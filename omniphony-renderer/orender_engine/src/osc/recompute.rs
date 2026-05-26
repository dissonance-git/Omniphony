use std::sync::Arc;

use renderer::live_params::RendererControl;
use runtime_control::band_topology_cache::BandTopologyCache;
use runtime_control::context::RuntimeControlContext;
use runtime_control::heatmap_sub::HeatmapSubscriptionState;
use runtime_control::osc::{BroadcastValue, republish_heatmap_if_changed};
use runtime_control::snapshot::{build_renderer_state_json, build_speakers_state_json};

use super::client_registry::OscClientRegistry;
use super::transport::{broadcast_int, broadcast_string};

pub(crate) fn trigger_layout_recompute(
    control: &Arc<RendererControl>,
    socket: &Arc<std::net::UdpSocket>,
    clients: &Arc<OscClientRegistry>,
    heatmap_sub: Arc<HeatmapSubscriptionState>,
    band_topology_cache: Arc<BandTopologyCache>,
) {
    if control.prepare_topology_rebuild().is_none() {
        log::warn!(
            "OSC apply: speaker positions cannot be updated — requested backend rebuild could not be prepared"
        );
        broadcast_int(socket, clients, "/omniphony/state/speakers/recomputing", 0);
        return;
    }

    if control
        .recomputing
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        log::warn!("OSC apply: VBAP recompute already in progress, ignoring");
        broadcast_int(socket, clients, "/omniphony/state/speakers/recomputing", 1);
        return;
    }

    let rebuild_plan = match control.prepare_topology_rebuild() {
        Some(plan) => plan,
        None => {
            log::warn!("OSC apply: failed to prepare render backend recompute plan");
            broadcast_int(socket, clients, "/omniphony/state/speakers/recomputing", 0);
            return;
        }
    };

    control
        .recomputing
        .store(true, std::sync::atomic::Ordering::Relaxed);
    broadcast_int(socket, clients, "/omniphony/state/speakers/recomputing", 1);

    let control_clone = Arc::clone(control);
    let socket_clone = Arc::clone(socket);
    let clients_clone = Arc::clone(clients);
    let rebuild_plan_for_thread = rebuild_plan.clone();
    let heatmap_sub_clone = heatmap_sub;
    let band_topology_cache_clone = band_topology_cache;

    std::thread::Builder::new()
        .name("render-backend-recompute".into())
        .spawn(move || {
            log::info!(
                "Render backend recompute started ({})",
                rebuild_plan_for_thread.log_summary()
            );
            match rebuild_plan_for_thread.build_topology() {
                Ok(new_topology) => {
                    control_clone.publish_topology(new_topology);
                    control_clone
                        .recomputing
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    // Layout/topology has changed → cached per-band topologies
                    // are stale. Drop them so the next heatmap subscribe rebuilds.
                    band_topology_cache_clone.clear();
                    log::info!(
                        "Render backend {} updated with new speaker layout",
                        rebuild_plan_for_thread.backend_id()
                    );
                    let renderer_state_json = {
                        let live = control_clone.live.read().unwrap();
                        let topology = control_clone.active_topology();
                        build_renderer_state_json(&live, &topology)
                    };
                    let layout_json = {
                        let layout = control_clone.editable_layout();
                        serde_json::to_string(&layout).unwrap_or_else(|_| "{}".to_string())
                    };
                    let speakers_state_json = {
                        let live = control_clone.live.read().unwrap();
                        let layout = control_clone.editable_layout();
                        build_speakers_state_json(&live, &layout)
                    };
                    broadcast_string(
                        &socket_clone,
                        &clients_clone,
                        "/omniphony/state/renderer",
                        &renderer_state_json,
                    );
                    broadcast_string(
                        &socket_clone,
                        &clients_clone,
                        "/omniphony/state/layout",
                        &layout_json,
                    );
                    broadcast_string(
                        &socket_clone,
                        &clients_clone,
                        "/omniphony/state/speakers",
                        &speakers_state_json,
                    );
                    broadcast_int(
                        &socket_clone,
                        &clients_clone,
                        "/omniphony/state/speakers/recomputing",
                        0,
                    );
                    // Push the new heatmap to the active subscription (if any)
                    // — but only if it actually differs from the cached one.
                    let ctx = RuntimeControlContext::with_shared_state(
                        Arc::clone(&control_clone),
                        heatmap_sub_clone,
                        band_topology_cache_clone,
                    );
                    let pushes = republish_heatmap_if_changed(&ctx);
                    for update in pushes {
                        if let BroadcastValue::String(value) = &update.value {
                            broadcast_string(&socket_clone, &clients_clone, &update.addr, value);
                        }
                    }
                    log::info!("Render backend recompute completed");
                }
                Err(e) => {
                    log::error!("Render backend recompute failed: {}", e);
                    control_clone
                        .recomputing
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    broadcast_int(
                        &socket_clone,
                        &clients_clone,
                        "/omniphony/state/speakers/recomputing",
                        0,
                    );
                }
            }
        })
        .expect("failed to spawn vbap-recompute thread");
}
