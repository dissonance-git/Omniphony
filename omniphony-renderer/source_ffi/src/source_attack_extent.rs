use renderer::source_scene::{SourceLaneKind, SourceSceneEvidence};

// These are renderer hypotheses, not historical source facts. The leading
// interval is intentionally short enough to anchor localization/attack while the
// slower release lets the recovered instrument grow into its normal FullSphere
// body after the onset. Physical listening may tune these constants later.
const ATTACK_COMPACT_SECONDS: f64 = 0.012;
const ATTACK_RELEASE_SECONDS: f64 = 0.024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct LaneAttackState {
    source_id: u64,
    initialized: bool,
    compact_remaining_frames: u32,
    release_pending: bool,
}

pub(crate) struct SourceAttackExtentTracker {
    compact_frames: u32,
    release_ramp_frames: u32,
    committed: Vec<LaneAttackState>,
    working: Vec<LaneAttackState>,
    extent_retention: Vec<f32>,
}

impl SourceAttackExtentTracker {
    pub(crate) fn new(sample_rate_hz: u32) -> Self {
        let frames = |seconds: f64| -> u32 {
            (seconds * sample_rate_hz as f64)
                .round()
                .clamp(1.0, u32::MAX as f64) as u32
        };
        Self {
            compact_frames: frames(ATTACK_COMPACT_SECONDS),
            release_ramp_frames: frames(ATTACK_RELEASE_SECONDS),
            committed: Vec::new(),
            working: Vec::new(),
            extent_retention: Vec::new(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.committed.clear();
        self.working.clear();
        self.extent_retention.clear();
    }

    /// Start a transactional render attempt. `commit()` is called only after the
    /// complete ABI block renders successfully, so a failed block cannot consume
    /// source-episode attack time that never sounded.
    pub(crate) fn begin(&mut self, sources: &[SourceSceneEvidence]) {
        self.working.clone_from(&self.committed);
        self.working.resize(sources.len(), LaneAttackState::default());
        self.extent_retention.resize(sources.len(), 1.0);
        for (lane_index, source) in sources.iter().enumerate() {
            self.observe_source(lane_index, *source);
        }
    }

    /// Apply an exact in-block source evidence transition before the following
    /// PCM segment is rendered.
    pub(crate) fn observe_source(&mut self, lane_index: usize, source: SourceSceneEvidence) {
        if lane_index >= self.working.len() {
            return;
        }
        let state = &mut self.working[lane_index];
        let source_id = source.source_id;

        // A renderer-local attack guard needs a bounded source episode. Zero is
        // explicitly "no usable episode token" and therefore earns no automatic
        // compactness. Shared wet fields never become point attacks.
        if source.lane_kind != SourceLaneKind::DrySource || source_id == 0 {
            state.source_id = source_id;
            state.initialized = source_id != 0;
            state.compact_remaining_frames = 0;
            state.release_pending = false;
            return;
        }

        if !state.initialized || state.source_id != source_id {
            state.source_id = source_id;
            state.initialized = true;
            state.compact_remaining_frames = self.compact_frames;
            state.release_pending = false;
        }
    }

    /// Fill the suppression-only sidecar consumed by SourceFrameRenderer.
    /// NativeRouting already closes source extent through policy; the tracker is
    /// still updated there so switching to FullSphere mid-episode does not invent
    /// a fresh attack.
    pub(crate) fn extent_retention(
        &mut self,
        sources: &[SourceSceneEvidence],
        full_sphere: bool,
    ) -> &[f32] {
        for (lane_index, source) in sources.iter().enumerate() {
            self.extent_retention[lane_index] = if full_sphere
                && source.lane_kind == SourceLaneKind::DrySource
                && self.working[lane_index].compact_remaining_frames != 0
            {
                0.0
            } else {
                1.0
            };
        }
        &self.extent_retention
    }

    /// Split only at the next time-based FullSphere compactness boundary. In
    /// NativeRouting the same episode clock advances, but it must not introduce
    /// a hidden audio segmentation because that mode already closes extent and
    /// should remain a chunk-neutral control.
    pub(crate) fn frames_until_transition(
        &self,
        sources: &[SourceSceneEvidence],
        maximum: usize,
        full_sphere: bool,
    ) -> usize {
        if !full_sphere {
            return maximum.max(1);
        }

        let mut frames = maximum;
        for (lane_index, source) in sources.iter().enumerate() {
            if source.lane_kind != SourceLaneKind::DrySource {
                continue;
            }
            let remaining = self.working[lane_index].compact_remaining_frames as usize;
            if remaining != 0 {
                frames = frames.min(remaining);
            }
        }
        frames.max(1).min(maximum.max(1))
    }

    pub(crate) fn ramp_frames(&self, ordinary_ramp_frames: u32) -> u32 {
        if self.working.iter().any(|state| state.release_pending) {
            ordinary_ramp_frames.max(self.release_ramp_frames)
        } else {
            ordinary_ramp_frames
        }
    }

    /// Advance only after a rendered subsegment succeeds. NativeRouting consumes
    /// the same real-time episode age but never arms a size-release ramp, because
    /// it never spent compact FullSphere extent in the first place.
    pub(crate) fn advance(&mut self, rendered_frames: usize, arm_release: bool) {
        let rendered_frames = rendered_frames.min(u32::MAX as usize) as u32;
        for state in &mut self.working {
            if state.compact_remaining_frames == 0 {
                continue;
            }
            if rendered_frames >= state.compact_remaining_frames {
                state.compact_remaining_frames = 0;
                state.release_pending = arm_release;
            } else {
                state.compact_remaining_frames -= rendered_frames;
            }
        }
    }

    /// The release ramp has been submitted to the renderer. Clear this marker
    /// after that subsegment succeeds; a failed attempt leaves committed state
    /// untouched and `begin()` reconstructs the pending transition next time.
    pub(crate) fn acknowledge_render(&mut self) {
        for state in &mut self.working {
            if state.compact_remaining_frames == 0 {
                state.release_pending = false;
            }
        }
    }

    pub(crate) fn commit(&mut self) {
        self.committed.clone_from(&self.working);
    }

    #[cfg(test)]
    fn compact_frames(&self) -> u32 {
        self.compact_frames
    }

    #[cfg(test)]
    fn release_ramp_frames(&self) -> u32 {
        self.release_ramp_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dry(source_id: u64) -> SourceSceneEvidence {
        SourceSceneEvidence {
            source_id,
            lane_kind: SourceLaneKind::DrySource,
            ..SourceSceneEvidence::default()
        }
    }

    fn wet(source_id: u64) -> SourceSceneEvidence {
        SourceSceneEvidence {
            source_id,
            lane_kind: SourceLaneKind::SharedWetReturn,
            ..SourceSceneEvidence::default()
        }
    }

    #[test]
    fn attack_windows_are_time_based_at_48khz() {
        let tracker = SourceAttackExtentTracker::new(48_000);
        assert_eq!(tracker.compact_frames(), 576);
        assert_eq!(tracker.release_ramp_frames(), 1_152);
    }

    #[test]
    fn new_dry_episode_is_point_like_then_releases() {
        let mut tracker = SourceAttackExtentTracker::new(48_000);
        let sources = [dry(10)];
        tracker.begin(&sources);
        assert_eq!(tracker.extent_retention(&sources, true), &[0.0]);
        assert_eq!(tracker.frames_until_transition(&sources, 2_048, true), 576);
        tracker.advance(576, true);
        assert_eq!(tracker.extent_retention(&sources, true), &[1.0]);
        assert_eq!(tracker.ramp_frames(96), 1_152);
        tracker.acknowledge_render();
        tracker.commit();

        tracker.begin(&sources);
        assert_eq!(tracker.extent_retention(&sources, true), &[1.0]);
        assert_eq!(tracker.ramp_frames(96), 96);
    }

    #[test]
    fn source_episode_change_rearms_without_changing_persistent_part_semantics() {
        let mut tracker = SourceAttackExtentTracker::new(48_000);
        let mut source = dry(10);
        source.persistent_part_id = Some(77);
        tracker.begin(&[source]);
        tracker.advance(576, true);
        tracker.acknowledge_render();
        tracker.commit();

        // A new bounded episode of the same persistent musical part gets a fresh
        // compact attack. Position continuity is handled independently by the
        // SourceFrameRenderer's persistent-part identity.
        source.source_id = 11;
        tracker.begin(&[source]);
        assert_eq!(tracker.extent_retention(&[source], true), &[0.0]);
    }

    #[test]
    fn shared_wet_never_becomes_a_point_attack() {
        let mut tracker = SourceAttackExtentTracker::new(48_000);
        let source = wet(99);
        tracker.begin(&[source]);
        assert_eq!(tracker.extent_retention(&[source], true), &[1.0]);
        assert_eq!(tracker.frames_until_transition(&[source], 512, true), 512);
    }

    #[test]
    fn native_mode_tracks_episode_without_splitting_or_release_ramp() {
        let mut tracker = SourceAttackExtentTracker::new(48_000);
        let source = dry(1);
        tracker.begin(&[source]);
        assert_eq!(tracker.extent_retention(&[source], false), &[1.0]);
        assert_eq!(tracker.frames_until_transition(&[source], 2_048, false), 2_048);
        tracker.advance(2_048, false);
        assert_eq!(tracker.ramp_frames(96), 96);
        tracker.commit();

        // The real-time episode clock was still consumed in NativeRouting, so a
        // later FullSphere switch does not manufacture a fresh compact attack.
        tracker.begin(&[source]);
        assert_eq!(tracker.extent_retention(&[source], true), &[1.0]);
    }

    #[test]
    fn failed_transaction_does_not_consume_attack_time() {
        let mut tracker = SourceAttackExtentTracker::new(48_000);
        let source = dry(5);
        tracker.begin(&[source]);
        tracker.advance(400, true);
        // no commit: model a renderer failure
        tracker.begin(&[source]);
        assert_eq!(tracker.frames_until_transition(&[source], 2_048, true), 576);
    }
}
