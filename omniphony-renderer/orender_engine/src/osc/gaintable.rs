//! Server-side cache of the serialized speaker gain table.
//!
//! Serializing + zlib-compressing the precomputed table costs a few MB of work,
//! so we do it once per topology and reuse the bytes for every subscriber push,
//! every NACK resend and every 5 s re-subscribe heartbeat. The cache is
//! invalidated on each topology rebuild (the table changed) and lazily refilled
//! on the next access — which also covers the "no precomputed table yet"
//! (realtime/polar) case, where serialization fails and we cache nothing.

use std::sync::{Arc, RwLock};

use runtime_control::context::RuntimeControlContext;
use runtime_control::osc::{gaintable_version, serialize_gaintable};

pub(crate) struct GaintableCache {
    inner: RwLock<Option<(u32, Arc<Vec<u8>>)>>,
}

impl GaintableCache {
    pub(crate) fn new() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }

    /// Drop the cached bytes (call on topology rebuild). The next [`ensure`]
    /// re-serializes from the now-current topology.
    pub(crate) fn invalidate(&self) {
        *self.inner.write().unwrap() = None;
    }

    /// Cached `(version, bytes)`, serializing + caching on a miss. Returns `None`
    /// when the active backend has no precomputed table (realtime evaluators),
    /// in which case the caller should reply `unavailable`.
    pub(crate) fn ensure(&self, ctx: &RuntimeControlContext) -> Option<(u32, Arc<Vec<u8>>)> {
        if let Some(cached) = self.inner.read().unwrap().clone() {
            return Some(cached);
        }
        let bytes = serialize_gaintable(ctx).ok()?;
        let version = gaintable_version(&bytes);
        let arc = Arc::new(bytes);
        *self.inner.write().unwrap() = Some((version, Arc::clone(&arc)));
        Some((version, arc))
    }
}
