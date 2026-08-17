#include <windows.h>

#include <cstdint>
#include <cstring>
#include <iostream>
#include <vector>

#include "vst2_minimal.h"

namespace {

intptr_t __cdecl host_callback(
    vst_effect_t*, int32_t, int32_t, int64_t, const char*, float) {
    return 0;
}

bool equal_bits(const std::vector<float>& a, const std::vector<float>& b) {
    return a.size() == b.size() &&
        std::memcmp(a.data(), b.data(), a.size() * sizeof(float)) == 0;
}

int fail(const char* message) {
    std::cerr << "FAIL: " << message << "\n";
    return 1;
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        return fail("usage: omniphony_vst_bridge_smoke <bridge.dll>");
    }

    HMODULE module = LoadLibraryW(argv[1]);
    if (module == nullptr) {
        return fail("bridge DLL did not load");
    }

    using MainFn = vst_effect_t*(__cdecl*)(vst_host_callback_t);
    using DiagnosticFn = uint32_t(*)();

    auto main_fn = reinterpret_cast<MainFn>(
        GetProcAddress(module, "VSTPluginMain"));
    auto ready_fn = reinterpret_cast<DiagnosticFn>(
        GetProcAddress(module, "omniphony_vst_bridge_backend_ready_instances"));
    auto abi_minor_fn = reinterpret_cast<DiagnosticFn>(
        GetProcAddress(module, "omniphony_vst_bridge_backend_abi_minor"));

    if (main_fn == nullptr || ready_fn == nullptr || abi_minor_fn == nullptr) {
        FreeLibrary(module);
        return fail("required bridge exports are missing");
    }

    vst_effect_t* effect = main_fn(host_callback);
    if (effect == nullptr || effect->magic_number != kVstMagic ||
        effect->process_float == nullptr ||
        effect->num_inputs != 2 || effect->num_outputs != 2 ||
        (effect->flags & kVstEffectFlagSupportsFloat) == 0) {
        FreeLibrary(module);
        return fail("VST effect contract is invalid");
    }

    effect->control(effect, kVstEffectInitialize, 0, 0, nullptr, 0.0f);
    const uint32_t abi_minor = abi_minor_fn();
    if (ready_fn() != 1 || abi_minor < 1) {
        effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
        FreeLibrary(module);
        return fail("Rust realtime backend was not loaded and instantiated");
    }

    effect->control(effect, kVstEffectSetSampleRate, 0, 0, nullptr, 48000.0f);
    effect->control(effect, kVstEffectSetBlockSize, 0, 256, nullptr, 0.0f);
    effect->control(effect, kVstEffectSuspend, 0, 1, nullptr, 0.0f);
    effect->control(effect, kVstEffectProcessBegin, 0, 0, nullptr, 0.0f);

    constexpr int32_t frames = 256;
    std::vector<float> left(frames);
    std::vector<float> right(frames);
    for (int32_t i = 0; i < frames; ++i) {
        left[i] = static_cast<float>((i * 17) % 113) / 113.0f - 0.5f;
        right[i] = static_cast<float>((i * 29) % 127) / 127.0f - 0.5f;
    }
    std::vector<float> out_left(frames, 99.0f);
    std::vector<float> out_right(frames, 99.0f);
    const float* input[2]{left.data(), right.data()};
    float* output[2]{out_left.data(), out_right.data()};

    effect->process_float(effect, input, output, frames);
    if (!equal_bits(left, out_left) || !equal_bits(right, out_right)) {
        effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
        FreeLibrary(module);
        return fail("identity backend changed samples");
    }

    constexpr int32_t oversized_frames = 300;
    std::vector<float> big_left(oversized_frames);
    std::vector<float> big_right(oversized_frames);
    for (int32_t i = 0; i < oversized_frames; ++i) {
        big_left[i] = static_cast<float>(i) / 1000.0f;
        big_right[i] = -static_cast<float>(i) / 997.0f;
    }
    std::vector<float> big_out_left(oversized_frames, 42.0f);
    std::vector<float> big_out_right(oversized_frames, 42.0f);
    const float* big_input[2]{big_left.data(), big_right.data()};
    float* big_output[2]{big_out_left.data(), big_out_right.data()};

    effect->process_float(
        effect, big_input, big_output, oversized_frames);
    if (!equal_bits(big_left, big_out_left) ||
        !equal_bits(big_right, big_out_right)) {
        effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
        FreeLibrary(module);
        return fail("oversized-block safety fallback was not passthrough");
    }

    effect->control(effect, kVstEffectProcessEnd, 0, 0, nullptr, 0.0f);
    effect->control(effect, kVstEffectSuspend, 0, 0, nullptr, 0.0f);
    effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);

    if (ready_fn() != 0) {
        FreeLibrary(module);
        return fail("backend instance leaked across VST destroy");
    }

    FreeLibrary(module);
    std::cout
        << "PASS: VST bridge loaded Rust ABI 0."
        << abi_minor
        << " and preserved stereo PCM bit-exactly\n";
    return 0;
}
