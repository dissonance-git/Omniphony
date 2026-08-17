#pragma once

// Minimal VST 2.x ABI surface used by the Omniphony Equalizer APO bootstrap.
//
// Field layout, constants, and naming are derived from the clean-room VST 2.x
// ABI reimplementation carried by Equalizer APO. See THIRD_PARTY_NOTICES.txt.
// This file intentionally contains only the subset required by the bridge.

#include <cstdint>

#define OMNI_VST_FOURCC(a, b, c, d) \
    ((static_cast<uint32_t>(a) << 24) | (static_cast<uint32_t>(b) << 16) | \
     (static_cast<uint32_t>(c) << 8) | static_cast<uint32_t>(d))

constexpr int32_t kVstMagic = static_cast<int32_t>(OMNI_VST_FOURCC('V', 's', 't', 'P'));
constexpr int32_t kVstVersion2400 = 2400;

constexpr int32_t kVstEffectFlagSupportsFloat = 1 << 4;

constexpr int32_t kVstEffectCategorySpatial = 0x05;

enum OmniphonyVstEffectOpcode : int32_t {
    kVstEffectInitialize = 0x00,
    kVstEffectDestroy = 0x01,
    kVstEffectSetSampleRate = 0x0A,
    kVstEffectSetBlockSize = 0x0B,
    kVstEffectSuspend = 0x0C,
    kVstEffectCategory = 0x23,
    kVstEffectBypass = 0x2C,
    kVstEffectName = 0x2D,
    kVstEffectVendorName = 0x2F,
    kVstEffectProductName = 0x30,
    kVstEffectVendorVersion = 0x31,
    kVstEffectSupports = 0x33,
    kVstEffectTailSamples = 0x34,
    kVstEffectVstVersion = 0x3A,
    kVstEffectProcessBegin = 0x47,
    kVstEffectProcessEnd = 0x48,
};

#pragma pack(push, 8)

struct vst_effect_t;

using vst_host_callback_t = intptr_t(__cdecl*)(
    vst_effect_t*, int32_t, int32_t, int64_t, const char*, float);
using vst_effect_control_t = intptr_t(__cdecl*)(
    vst_effect_t*, int32_t, int32_t, intptr_t, void*, float);
using vst_effect_process_t = void(__cdecl*)(
    vst_effect_t*, const float* const*, float**, int32_t);
using vst_effect_set_parameter_t = void(__cdecl*)(
    vst_effect_t*, uint32_t, float);
using vst_effect_get_parameter_t = float(__cdecl*)(
    vst_effect_t*, uint32_t);
using vst_effect_process_float_t = void(__cdecl*)(
    vst_effect_t*, const float* const*, float**, int32_t);
using vst_effect_process_double_t = void(__cdecl*)(
    vst_effect_t*, const double* const*, double**, int32_t);

struct vst_effect_t {
    int32_t magic_number;
    vst_effect_control_t control;
    vst_effect_process_t process;
    vst_effect_set_parameter_t set_parameter;
    vst_effect_get_parameter_t get_parameter;
    int32_t num_programs;
    int32_t num_params;
    int32_t num_inputs;
    int32_t num_outputs;
    int32_t flags;
    void* reserved_0;
    void* reserved_1;
    int32_t delay;
    int32_t reserved_2;
    int32_t reserved_3;
    float input_output_ratio;
    void* effect_internal;
    void* host_internal;
    int32_t unique_id;
    int32_t version;
    vst_effect_process_float_t process_float;
    vst_effect_process_double_t process_double;
    uint8_t reserved_tail[56];
};

#pragma pack(pop)

static_assert(sizeof(void*) == 8, "personal bootstrap is x64-only");
