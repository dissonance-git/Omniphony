//! Cache of `RenderTopology` instances built per band.
//!
//! Building a band-specific topology (`prepare_topology_rebuild_for_layout` +
//! `build_topology`) regenerates the cartesian VBAP cache, which is the
//! dominant cost when the studio asks for a heatmap. The result depends only
//! on the band composition (and the main layout's radius/options), not on the
//! selected speaker or its position. So we can keep the topologies around and
//! invalidate them only when the main layout actually changes (i.e. on a
//! successful `publish_topology`).
//!
//! Hit path: subscribing to a different speaker in the same band, or
//! resubscribing to the same speaker after edits that did not trigger a
//! recompute, returns instantly.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use renderer::live_params::RenderTopology;

#[derive(Default)]
pub struct BandTopologyCache {
    inner: RwLock<HashMap<usize, Arc<RenderTopology>>>,
}

impl BandTopologyCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, band_index: usize) -> Option<Arc<RenderTopology>> {
        self.inner.read().unwrap().get(&band_index).cloned()
    }

    pub fn insert(&self, band_index: usize, topology: Arc<RenderTopology>) {
        self.inner.write().unwrap().insert(band_index, topology);
    }

    pub fn clear(&self) {
        self.inner.write().unwrap().clear();
    }
}
