#ifndef OMNIPHONY_REALTIME_H
#define OMNIPHONY_REALTIME_H

#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define OMNIPHONY_REALTIME_ABI_MAJOR 0
#define OMNIPHONY_REALTIME_ABI_MINOR 1

typedef struct OmniphonyRealtimeProcessor OmniphonyRealtimeProcessor;

typedef struct OmniphonyRealtimeConfig {
    uint32_t sample_rate_hz;
    uint32_t channels;
} OmniphonyRealtimeConfig;

uint32_t omniphony_realtime_abi_major(void);
uint32_t omniphony_realtime_abi_minor(void);

OmniphonyRealtimeProcessor *omniphony_realtime_create(
    const OmniphonyRealtimeConfig *config);

void omniphony_realtime_destroy(OmniphonyRealtimeProcessor *processor);

int32_t omniphony_realtime_reset(OmniphonyRealtimeProcessor *processor);

/*
 * Process interleaved float32 PCM. Input/output may alias for in-place audio
 * processing. Returns 0 on success and a negative error code for invalid input.
 *
 * ABI minor 1 is deliberately exact identity. Native Windows integration can
 * prove transport, callback and failure behavior against this oracle before the
 * protected Omniphony binaural renderer is attached behind the same boundary.
 */
int32_t omniphony_realtime_process_f32(
    OmniphonyRealtimeProcessor *processor,
    const float *input,
    float *output,
    size_t frames);

uint32_t omniphony_realtime_sample_rate_hz(
    const OmniphonyRealtimeProcessor *processor);
uint32_t omniphony_realtime_channels(
    const OmniphonyRealtimeProcessor *processor);

#ifdef __cplusplus
}
#endif

#endif
