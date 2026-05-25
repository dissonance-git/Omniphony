#ifndef ORENDER_H
#define ORENDER_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Opaque handle to a decode→render session. Created by [`orender_create`],
 * freed by [`orender_destroy`]. Internally a boxed [`Engine`].
 */
typedef struct OrenderRenderer {
  uint8_t _private[0];
} OrenderRenderer;

/**
 * Session configuration passed to [`orender_create`]. All `*const c_char`
 * fields are UTF-8, nul-terminated, and may be NULL (treated as "unset").
 */
typedef struct OrenderConfig {
  /**
   * Output/host sample rate in Hz. 0 → 48000.
   */
  uint32_t sample_rate;
  /**
   * Path to the omniphony YAML config (drives the speaker layout + all render
   * params). NULL → built-in defaults.
   */
  const char *config_yaml_path;
  /**
   * Optional speaker-layout YAML path overriding the config. NULL → use the
   * config's embedded layout, else the 7.1.4 preset.
   */
  const char *speaker_layout_path;
  /**
   * Path to the decoder bridge plugin (e.g. truehd_bridge.so). REQUIRED:
   * library hosts cannot use the exe-relative search.
   */
  const char *bridge_path;
  /**
   * Enable the OSC live-control server. (Not yet wired in this build.)
   */
  int osc_enabled;
  /**
   * Incoming OSC port (0 = auto).
   */
  uint16_t osc_port_in;
  /**
   * Outgoing/monitoring OSC port.
   */
  uint16_t osc_port_out;
  /**
   * OSC bind address (default "127.0.0.1").
   */
  const char *osc_bind;
  /**
   * OSC monitoring target host.
   */
  const char *osc_host;
} OrenderConfig;

/**
 * Create a session. Returns NULL on failure (bad config, missing bridge, etc.).
 */
struct OrenderRenderer *orender_create(const struct OrenderConfig *cfg);

/**
 * Free a session created by [`orender_create`]. NULL is ignored.
 */
void orender_destroy(struct OrenderRenderer *r);

/**
 * 1 if the current presentation may contain spatial objects (Atmos), 0 if not
 * (plain TrueHD — the host should fall back to its standard decoder), <0 on
 * error. Meaningful after at least one [`orender_process`] call.
 */
int orender_is_spatial(const struct OrenderRenderer *r);

/**
 * Number of output channels (speakers) the renderer produces, 0 on error.
 */
uint32_t orender_channel_count(const struct OrenderRenderer *r);

/**
 * Reset after a seek/discontinuity (flushes decoder + renderer state, keeps
 * live params).
 */
void orender_reset(struct OrenderRenderer *r);

/**
 * Push one raw TrueHD packet and render whatever frames it yields.
 *
 * The caller owns `out` (capacity `out_cap_samples` floats). On success the
 * rendered interleaved samples are written there and `*out_frames` /
 * `*out_channels` / `*out_pts_us` are set.
 *
 * Returns: 0 = OK (may be 0 frames — need more data), >0 = output buffer too
 * small (nothing written; retry with a larger buffer), <0 = error.
 */
int orender_process(struct OrenderRenderer *r,
                    const uint8_t *pkt,
                    uintptr_t pkt_len,
                    int64_t _pts_us,
                    float *out,
                    uintptr_t out_cap_samples,
                    uintptr_t *out_frames,
                    uint32_t *out_channels,
                    int64_t *out_pts_us);

/**
 * ABI major version. A bump means a breaking change (new soname).
 */
uint32_t orender_version_major(void);

/**
 * ABI minor version (backwards-compatible additions).
 */
uint32_t orender_version_minor(void);

#endif  /* ORENDER_H */
