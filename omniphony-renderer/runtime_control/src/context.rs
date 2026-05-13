use std::sync::Arc;

use audio_input::InputControl;
use audio_output::AudioControl;
use renderer::live_params::RendererControl;

use crate::band_topology_cache::BandTopologyCache;
use crate::heatmap_sub::HeatmapSubscriptionState;

#[derive(Clone)]
pub struct RuntimeControlContext {
    pub renderer: Arc<RendererControl>,
    pub audio: Option<Arc<AudioControl>>,
    pub input: Option<Arc<InputControl>>,
    pub heatmap_sub: Arc<HeatmapSubscriptionState>,
    pub band_topology_cache: Arc<BandTopologyCache>,
}

impl RuntimeControlContext {
    pub fn new(
        renderer: Arc<RendererControl>,
        audio: Option<Arc<AudioControl>>,
        input: Option<Arc<InputControl>>,
    ) -> Self {
        Self {
            renderer,
            audio,
            input,
            heatmap_sub: Arc::new(HeatmapSubscriptionState::new()),
            band_topology_cache: Arc::new(BandTopologyCache::new()),
        }
    }

    /// Create a context that shares the heatmap subscription + band-topology
    /// cache with other contexts (used by the OSC dispatcher to keep this
    /// state alive across per-message context creations).
    pub fn with_shared_state(
        renderer: Arc<RendererControl>,
        audio: Option<Arc<AudioControl>>,
        input: Option<Arc<InputControl>>,
        heatmap_sub: Arc<HeatmapSubscriptionState>,
        band_topology_cache: Arc<BandTopologyCache>,
    ) -> Self {
        Self {
            renderer,
            audio,
            input,
            heatmap_sub,
            band_topology_cache,
        }
    }
}
