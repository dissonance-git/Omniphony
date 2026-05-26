use std::sync::Arc;

use renderer::live_params::RendererControl;

use crate::band_topology_cache::BandTopologyCache;
use crate::heatmap_sub::HeatmapSubscriptionState;

/// Per-message context for the core OSC dispatch (`apply_simple_osc_control`).
/// The audio-free core holds the renderer control + shared heatmap/topology
/// caches; audio-output/audio-input live in `host_audio::HostAudio` and reach
/// the OSC server via [`crate::HostControlHandler`].
#[derive(Clone)]
pub struct RuntimeControlContext {
    pub renderer: Arc<RendererControl>,
    pub heatmap_sub: Arc<HeatmapSubscriptionState>,
    pub band_topology_cache: Arc<BandTopologyCache>,
}

impl RuntimeControlContext {
    pub fn new(renderer: Arc<RendererControl>) -> Self {
        Self {
            renderer,
            heatmap_sub: Arc::new(HeatmapSubscriptionState::new()),
            band_topology_cache: Arc::new(BandTopologyCache::new()),
        }
    }

    /// Create a context that shares the heatmap subscription + band-topology
    /// cache with other contexts (used by the OSC dispatcher to keep this
    /// state alive across per-message context creations).
    pub fn with_shared_state(
        renderer: Arc<RendererControl>,
        heatmap_sub: Arc<HeatmapSubscriptionState>,
        band_topology_cache: Arc<BandTopologyCache>,
    ) -> Self {
        Self {
            renderer,
            heatmap_sub,
            band_topology_cache,
        }
    }
}
