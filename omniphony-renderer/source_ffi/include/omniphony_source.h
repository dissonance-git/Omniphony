#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OmniphonySourceProcessor OmniphonySourceProcessor;

enum {
    OMNIPHONY_SOURCE_FLAG_PERSISTENT_PART = 1u << 0,
    OMNIPHONY_SOURCE_FLAG_NATIVE_STEREO_ROUTE = 1u << 1,
    OMNIPHONY_SOURCE_FLAG_AUTHORED_POSITION = 1u << 2,
    /*
     * The host has already applied this source's native sample-accurate gain
     * trajectory to input PCM. left_gain/right_gain remain pose/polarity
     * evidence and MUST NOT be scalar-applied again by Omniphony.
     */
    OMNIPHONY_SOURCE_FLAG_ROUTE_GAIN_PREAPPLIED = 1u << 3,
};

enum {
    OMNIPHONY_SOURCE_LANE_DRY = 0,
    OMNIPHONY_SOURCE_LANE_SHARED_WET = 1,
    OMNIPHONY_SOURCE_LANE_REFERENCE_MIX = 2,
};

enum {
    OMNIPHONY_SOURCE_SPATIAL_NATIVE_ROUTING = 0,
    OMNIPHONY_SOURCE_SPATIAL_FULL_SPHERE = 1,
};

enum {
    OMNIPHONY_SOURCE_HRIR_SAF_KEMAR = 0,
    OMNIPHONY_SOURCE_HRIR_SYNTHETIC = 1,
};

typedef struct OmniphonySourceConfig {
    uint32_t sample_rate_hz;
    uint32_t spatial_mode;
    uint32_t externalization;
    uint32_t hrir_source;
    float unit_scale_m;
    float reflection_level;
} OmniphonySourceConfig;

typedef struct OmniphonySourceEvidenceV1 {
    uint32_t lane_kind;
    uint32_t flags;
    uint64_t source_id;
    uint64_t persistent_part_id;
    float left_gain;
    float right_gain;
    float authored_x;
    float authored_y;
    float authored_z;
    float foundation;
    float foreground;
    float diffuse;
    float width;
    float vertical_affinity;
    float confidence;
} OmniphonySourceEvidenceV1;

/*
 * One evidence state change inside the current audio block. frame_offset is a
 * zero-based sample/frame offset relative to the block passed to
 * omniphony_source_process_events_f32(). Events must be ordered by nondecreasing
 * frame_offset. Multiple lane changes may share one boundary.
 */
typedef struct OmniphonySourceEvidenceEventV1 {
    uint32_t frame_offset;
    uint32_t lane_index;
    OmniphonySourceEvidenceV1 evidence;
} OmniphonySourceEvidenceEventV1;

uint32_t omniphony_source_abi_major(void);
uint32_t omniphony_source_abi_minor(void);

OmniphonySourceProcessor *omniphony_source_create(const OmniphonySourceConfig *config);
void omniphony_source_destroy(OmniphonySourceProcessor *processor);
int32_t omniphony_source_reset(OmniphonySourceProcessor *processor);
int32_t omniphony_source_set_spatial_mode(OmniphonySourceProcessor *processor, uint32_t mode);
int32_t omniphony_source_set_externalization(OmniphonySourceProcessor *processor, uint32_t enabled);

/*
 * Render interleaved causal source lanes to interleaved stereo f32.
 *
 * input:        frames * source_count samples
 * sources:      one evidence record per source channel, same order as input
 * output:       frames * 2 samples
 * sample_pos:   absolute source-frame position for metadata/ramp continuity
 * ramp_frames:  presentation movement ramp in source frames
 *
 * OMNIPHONY_SOURCE_LANE_REFERENCE_MIX is a protected control and is rejected
 * here. Keep the historical/reference stereo mix outside the object-lane call
 * for A/B and reconstruction validation.
 *
 * This legacy entry point is equivalent to omniphony_source_process_events_f32
 * with no timed events.
 */
int32_t omniphony_source_process_f32(
    OmniphonySourceProcessor *processor,
    const float *input,
    const OmniphonySourceEvidenceV1 *sources,
    size_t source_count,
    size_t frames,
    uint64_t sample_pos,
    uint32_t ramp_frames,
    float *output);

/*
 * Render the same causal source block while applying ordered source-evidence
 * changes at exact frame boundaries inside the buffer. The renderer processes
 * audio only up to the next event boundary, applies all events at that boundary,
 * then continues. No whole-track automation or future-song knowledge is used.
 *
 * Events at frame_offset == frames are accepted as a zero-length terminal
 * state transition; callers should normally carry that evidence as the initial
 * state of the next block. Malformed event lists are rejected before any audio
 * from this call is rendered.
 */
int32_t omniphony_source_process_events_f32(
    OmniphonySourceProcessor *processor,
    const float *input,
    const OmniphonySourceEvidenceV1 *sources,
    size_t source_count,
    const OmniphonySourceEvidenceEventV1 *events,
    size_t event_count,
    size_t frames,
    uint64_t sample_pos,
    uint32_t ramp_frames,
    float *output);

#ifdef __cplusplus
}
#endif
