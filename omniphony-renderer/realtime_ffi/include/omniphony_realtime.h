#ifndef OMNIPHONY_REALTIME_H
#define OMNIPHONY_REALTIME_H

#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define OMNIPHONY_REALTIME_ABI_MAJOR 0
#define OMNIPHONY_REALTIME_ABI_MINOR 3

#define OMNIPHONY_REALTIME_MODE_IDENTITY 0u
#define OMNIPHONY_REALTIME_MODE_CURRENT 1u

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

int32_t omniphony_realtime_set_mode(
    OmniphonyRealtimeProcessor *processor,
    uint32_t mode);

uint32_t omniphony_realtime_mode(
    const OmniphonyRealtimeProcessor *processor);

uint64_t omniphony_realtime_processed_blocks(
    const OmniphonyRealtimeProcessor *processor);

/*
 * Fixed host delay, in frames, for the active processing mode. Identity is 0.
 * Current uses a bounded delayed-dry safety lane so worker underruns never turn
 * into time-shifted immediate dry audio.
 */
size_t omniphony_realtime_latency_frames(
    const OmniphonyRealtimeProcessor *processor);

int32_t omniphony_realtime_reset(OmniphonyRealtimeProcessor *processor);

/*
 * Process interleaved float32 PCM. Input/output may alias for in-place audio
 * processing. Returns 0 on success and a negative error code for invalid input.
 *
 * Mode 0 is exact identity and remains the deterministic transport oracle.
 * Mode 1 runs the retained stereo Current model on a dedicated worker thread;
 * the host callback itself only performs bounded PCM movement through
 * preallocated rings. Current includes the primary listener's separate Noire X
 * output-correction profile before the retained final linked peak guard.
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
