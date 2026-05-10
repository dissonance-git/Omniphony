//! Speaker heatmap subscription state.
//!
//! Pub/sub model: a single client (the studio) subscribes to one speaker at a
//! time. The renderer holds the latest payload hash and only re-broadcasts when
//! the recomputed payload differs from the last one sent. This replaces the old
//! pull model where the studio re-requested the heatmap on every state echo,
//! which fed back into a heatmap storm and saturated the renderer.

use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct HeatmapSubscription {
    pub speaker_index: usize,
    pub band_index: usize,
    pub modes: Vec<String>, // subset of {"slices", "volume"}
    pub max_samples: usize,
}

#[derive(Default)]
pub struct HeatmapSubscriptionState {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    current: Option<HeatmapSubscription>,
    last_hash: Option<u64>,
}

impl HeatmapSubscriptionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, sub: HeatmapSubscription) {
        let mut inner = self.inner.lock().unwrap();
        inner.current = Some(sub);
        inner.last_hash = None; // force first broadcast
    }

    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.current = None;
        inner.last_hash = None;
    }

    pub fn current(&self) -> Option<HeatmapSubscription> {
        self.inner.lock().unwrap().current.clone()
    }

    /// Update the cached hash. Returns true if it changed (and a broadcast is
    /// warranted), false if the hash matched the previous one.
    pub fn update_hash_if_changed(&self, new_hash: u64) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if inner.last_hash == Some(new_hash) {
            false
        } else {
            inner.last_hash = Some(new_hash);
            true
        }
    }
}
