//! Worker-side bridge from Windows Spatial Audio static objects into the
//! existing source-aware Omniphony renderer.
//!
//! This module deliberately starts with static objects only. A Windows spatial
//! render stream declares its requested static-object mask at activation, so the
//! topology is fixed for the stream lifetime. Locking that topology lets the
//! portable renderer preserve channel history without pretending that dynamic
//! object allocation/lifetime has already been solved.

use crate::StereoLookaheadPeakGuard;
use crate::noire_x_profile::NoireXPersonalEq;
use crate::windows_spatial_contract::{
    WindowsStaticObject, WindowsStaticObjectRole,
};
use orender_engine::{
    SourceRendererOptions, SourceSpatialMode, build_source_frame_renderer,
};
use renderer::source_frame::SourceFrameRenderer;
use renderer::source_scene::{SourceLaneKind, SourceSceneEvidence};
use std::f32::consts::PI;

const STATIC_OBJECT_OUTPUT_GAIN: f32 = 0.90;
const LFE_CUTOFF_HZ: f32 = 120.0;
const STATIC_SOURCE_NAMESPACE: u64 = 0x5354_4154_4943_0000;

fn static_source_id(role: WindowsStaticObjectRole) -> u64 {
    STATIC_SOURCE_NAMESPACE | role.canonical_scene_index() as u64
}

#[derive(Clone, Copy, Debug)]
struct OnePoleLowPass {
    alpha: f32,
    state: f32,
}

impl OnePoleLowPass {
    fn new(sample_rate_hz: u32, cutoff_hz: f32) -> Self {
        let rate = sample_rate_hz.max(1) as f32;
        let alpha = 1.0 - (-2.0 * PI * cutoff_hz.max(1.0) / rate).exp();
        Self { alpha, state: 0.0 }
    }

    fn process(&mut self, sample: f32) -> f32 {
        let input = if sample.is_finite() { sample } else { 0.0 };
        self.state += self.alpha * (input - self.state);
        self.state
    }
}

struct LfeLowPass {
    first: OnePoleLowPass,
    second: OnePoleLowPass,
}

impl LfeLowPass {
    fn new(sample_rate_hz: u32) -> Self {
        Self {
            first: OnePoleLowPass::new(sample_rate_hz, LFE_CUTOFF_HZ),
            second: OnePoleLowPass::new(sample_rate_hz, LFE_CUTOFF_HZ),
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        self.second.process(self.first.process(sample))
    }
}

/// Fixed static-object topology for one Windows Spatial Audio render stream.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StaticTopology {
    directional_roles: Vec<WindowsStaticObjectRole>,
    has_lfe: bool,
}

impl StaticTopology {
    fn from_objects(objects: &[WindowsStaticObject<'_>]) -> Result<Self, String> {
        let mut seen = [false; 17];
        let mut directional_roles = Vec::with_capacity(objects.len());
        let mut has_lfe = false;

        for object in objects {
            let index = object.role.canonical_scene_index();
            if seen[index] {
                return Err(format!("duplicate static object role {:?}", object.role));
            }
            seen[index] = true;
            if object.role == WindowsStaticObjectRole::LowFrequency {
                has_lfe = true;
            } else {
                if object.windows_position.is_none() {
                    return Err(format!(
                        "directional static object {:?} has no endpoint position",
                        object.role
                    ));
                }
                directional_roles.push(object.role);
            }
        }

        directional_roles.sort_by_key(|role| role.canonical_scene_index());
        Ok(Self {
            directional_roles,
            has_lfe,
        })
    }
}

/// Allocating renderer work for Spatial Audio runs here, never in the Windows
/// object callback itself. A future host ABI should enqueue complete quanta to a
/// worker that owns this pipeline.
pub(crate) struct WindowsStaticObjectPipeline {
    renderer: SourceFrameRenderer,
    topology: Option<StaticTopology>,
    sources: Vec<SourceSceneEvidence>,
    interleaved: Vec<f32>,
    render_buf: Vec<f32>,
    lfe: LfeLowPass,
    headphone_eq: NoireXPersonalEq,
    peak_guard: StereoLookaheadPeakGuard,
    sample_pos: u64,
}

impl WindowsStaticObjectPipeline {
    pub(crate) fn new(sample_rate_hz: u32) -> Result<Self, String> {
        let renderer = build_source_frame_renderer(
            sample_rate_hz,
            None,
            SourceRendererOptions {
                mode: SourceSpatialMode::FullSphere,
                externalization: false,
                ..SourceRendererOptions::default()
            },
        )
        .map_err(|error| error.to_string())?;

        Ok(Self {
            renderer,
            topology: None,
            sources: Vec::new(),
            interleaved: Vec::new(),
            render_buf: Vec::new(),
            lfe: LfeLowPass::new(sample_rate_hz),
            headphone_eq: NoireXPersonalEq::new(sample_rate_hz),
            peak_guard: StereoLookaheadPeakGuard::new(sample_rate_hz),
            sample_pos: 0,
        })
    }

    fn validate_quantum(
        &self,
        objects: &[WindowsStaticObject<'_>],
    ) -> Result<(StaticTopology, usize), String> {
        if objects.is_empty() {
            return Err("static object quantum is empty".to_string());
        }
        let topology = StaticTopology::from_objects(objects)?;
        let frames = objects[0].mono_pcm.len();
        if frames == 0 {
            return Err("static object quantum has zero frames".to_string());
        }
        if objects.iter().any(|object| object.mono_pcm.len() != frames) {
            return Err("static object PCM frame counts differ within one quantum".to_string());
        }
        if let Some(expected) = &self.topology {
            if expected != &topology {
                return Err(format!(
                    "static object topology changed inside one stream: expected {expected:?}, got {topology:?}"
                ));
            }
        }
        Ok((topology, frames))
    }

    fn object_for_role<'a>(
        objects: &'a [WindowsStaticObject<'a>],
        role: WindowsStaticObjectRole,
    ) -> Option<&'a WindowsStaticObject<'a>> {
        objects.iter().find(|object| object.role == role)
    }

    fn rebuild_sources(
        &mut self,
        objects: &[WindowsStaticObject<'_>],
        topology: &StaticTopology,
    ) -> Result<(), String> {
        self.sources.clear();
        self.sources.reserve(topology.directional_roles.len());
        for &role in &topology.directional_roles {
            let object = Self::object_for_role(objects, role)
                .ok_or_else(|| format!("missing static object role {role:?}"))?;
            let position = object
                .omniphony_position()
                .ok_or_else(|| format!("static object role {role:?} lost authored position"))?;
            self.sources.push(SourceSceneEvidence {
                lane_kind: SourceLaneKind::DrySource,
                source_id: static_source_id(role),
                persistent_part_id: Some(static_source_id(role)),
                authored_position: Some([
                    position[0] as f64,
                    position[1] as f64,
                    position[2] as f64,
                ]),
                confidence: 1.0,
                ..SourceSceneEvidence::default()
            });
        }
        Ok(())
    }

    /// Render one complete Windows static-object update quantum to binaural
    /// stereo. Object PCM remains mono and source-separated until this function
    /// interleaves the active directional lanes for `SourceFrameRenderer`.
    pub(crate) fn process(
        &mut self,
        objects: &[WindowsStaticObject<'_>],
    ) -> Result<Vec<f32>, String> {
        let (topology, frames) = self.validate_quantum(objects)?;
        self.rebuild_sources(objects, &topology)?;

        self.interleaved.clear();
        self.interleaved
            .reserve(frames.saturating_mul(topology.directional_roles.len()));
        if !topology.directional_roles.is_empty() {
            for frame_index in 0..frames {
                for &role in &topology.directional_roles {
                    let object = Self::object_for_role(objects, role)
                        .ok_or_else(|| format!("missing static object role {role:?}"))?;
                    let sample = object.mono_pcm[frame_index];
                    self.interleaved
                        .push(if sample.is_finite() { sample } else { 0.0 });
                }
            }
        }

        let mut mixed = if topology.directional_roles.is_empty() {
            vec![0.0f32; frames * 2]
        } else {
            let rendered = self
                .renderer
                .render_source_frame_with_gain_policy(
                    &self.interleaved,
                    &self.sources,
                    None,
                    self.sample_pos,
                    0,
                    std::mem::take(&mut self.render_buf),
                    false,
                )
                .map_err(|error| error.to_string())?;
            rendered.samples
        };

        if mixed.len() != frames * 2 {
            return Err(format!(
                "static object renderer returned {} samples for {frames} frames",
                mixed.len()
            ));
        }

        if topology.has_lfe {
            let lfe = Self::object_for_role(objects, WindowsStaticObjectRole::LowFrequency)
                .ok_or_else(|| "LFE topology declared but LFE object is absent".to_string())?;
            for (frame_index, &sample) in lfe.mono_pcm.iter().enumerate() {
                let low = self.lfe.process(sample);
                mixed[frame_index * 2] += low;
                mixed[frame_index * 2 + 1] += low;
            }
        }

        for sample in &mut mixed {
            *sample = if sample.is_finite() {
                *sample * STATIC_OBJECT_OUTPUT_GAIN
            } else {
                0.0
            };
        }

        self.headphone_eq.process_interleaved(&mut mixed);
        self.sample_pos = self.sample_pos.saturating_add(frames as u64);
        self.topology.get_or_insert(topology);
        Ok(self.peak_guard.process_interleaved(&mixed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::windows_spatial_contract::WindowsSpatialPosition;

    fn object<'a>(
        role: WindowsStaticObjectRole,
        position: Option<WindowsSpatialPosition>,
        pcm: &'a [f32],
    ) -> WindowsStaticObject<'a> {
        WindowsStaticObject {
            role,
            windows_position: position,
            mono_pcm: pcm,
        }
    }

    #[test]
    fn topology_is_canonical_and_rejects_duplicates() {
        let pcm = [0.0f32; 8];
        let objects = [
            object(
                WindowsStaticObjectRole::TopFrontRight,
                Some(WindowsSpatialPosition::new(0.5, 0.7, -0.5)),
                &pcm,
            ),
            object(
                WindowsStaticObjectRole::FrontLeft,
                Some(WindowsSpatialPosition::new(-0.7, 0.0, -0.7)),
                &pcm,
            ),
        ];
        let topology = StaticTopology::from_objects(&objects).unwrap();
        assert_eq!(
            topology.directional_roles,
            vec![
                WindowsStaticObjectRole::FrontLeft,
                WindowsStaticObjectRole::TopFrontRight,
            ]
        );

        let duplicate = [objects[0], objects[0]];
        assert!(StaticTopology::from_objects(&duplicate).is_err());
    }

    #[test]
    fn directional_static_role_requires_endpoint_geometry() {
        let pcm = [0.0f32; 8];
        let objects = [object(WindowsStaticObjectRole::FrontLeft, None, &pcm)];
        assert!(StaticTopology::from_objects(&objects).is_err());
    }

    #[test]
    fn lfe_is_non_directional_and_topology_tracks_it_separately() {
        let pcm = [0.0f32; 8];
        let objects = [object(
            WindowsStaticObjectRole::LowFrequency,
            Some(WindowsSpatialPosition::new(9.0, 9.0, 9.0)),
            &pcm,
        )];
        let topology = StaticTopology::from_objects(&objects).unwrap();
        assert!(topology.directional_roles.is_empty());
        assert!(topology.has_lfe);
    }

    #[test]
    fn static_source_ids_are_namespaced_and_stable() {
        let front = static_source_id(WindowsStaticObjectRole::FrontLeft);
        let top = static_source_id(WindowsStaticObjectRole::TopFrontLeft);
        assert_ne!(front, top);
        assert_eq!(front, static_source_id(WindowsStaticObjectRole::FrontLeft));
        assert_eq!(front & 0xFFFF_FFFF_FFFF_0000, STATIC_SOURCE_NAMESPACE);
    }
}
