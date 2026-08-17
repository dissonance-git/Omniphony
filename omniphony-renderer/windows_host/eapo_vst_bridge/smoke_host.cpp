#include <windows.h>

#include <cmath>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <vector>

#include "vst2_minimal.h"

namespace {
intptr_t __cdecl host_callback(vst_effect_t*, int32_t, int32_t, int64_t, const char*, float) { return 0; }
bool equal_bits(const std::vector<float>& a, const std::vector<float>& b) {
    return a.size() == b.size() && std::memcmp(a.data(), b.data(), a.size() * sizeof(float)) == 0;
}
int fail(const char* message) { std::cerr << "FAIL: " << message << "\n"; return 1; }
}

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) return fail("usage: omniphony_vst_bridge_smoke <bridge.dll>");
    HMODULE module = LoadLibraryW(argv[1]);
    if (module == nullptr) return fail("bridge DLL did not load");

    using MainFn = vst_effect_t*(__cdecl*)(vst_host_callback_t);
    using U32Fn = uint32_t(*)();
    using U64Fn = uint64_t(*)();
    auto main_fn = reinterpret_cast<MainFn>(GetProcAddress(module, "VSTPluginMain"));
    auto ready_fn = reinterpret_cast<U32Fn>(GetProcAddress(module, "omniphony_vst_bridge_backend_ready_instances"));
    auto current_fn = reinterpret_cast<U32Fn>(GetProcAddress(module, "omniphony_vst_bridge_backend_current_instances"));
    auto abi_minor_fn = reinterpret_cast<U32Fn>(GetProcAddress(module, "omniphony_vst_bridge_backend_abi_minor"));
    auto blocks_fn = reinterpret_cast<U64Fn>(GetProcAddress(module, "omniphony_vst_bridge_processed_blocks"));
    if (main_fn == nullptr || ready_fn == nullptr || current_fn == nullptr || abi_minor_fn == nullptr || blocks_fn == nullptr) {
        FreeLibrary(module);
        return fail("required bridge exports are missing");
    }

    vst_effect_t* effect = main_fn(host_callback);
    if (effect == nullptr || effect->magic_number != kVstMagic || effect->process_float == nullptr ||
        effect->num_inputs != 2 || effect->num_outputs != 2 || (effect->flags & kVstEffectFlagSupportsFloat) == 0) {
        FreeLibrary(module);
        return fail("VST effect contract is invalid");
    }

    effect->control(effect, kVstEffectInitialize, 0, 0, nullptr, 0.0f);
    if (ready_fn() != 1 || abi_minor_fn() < 2 || current_fn() != 0) {
        effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
        FreeLibrary(module);
        return fail("identity bootstrap did not initialize ABI 0.2 cleanly");
    }

    effect->control(effect, kVstEffectSetSampleRate, 0, 0, nullptr, 48000.0f);
    effect->control(effect, kVstEffectSetBlockSize, 0, 256, nullptr, 0.0f);
    if (ready_fn() != 1 || current_fn() != 1) {
        effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
        FreeLibrary(module);
        return fail("Current-model worker did not activate at the explicit host rate");
    }
    effect->control(effect, kVstEffectProcessBegin, 0, 0, nullptr, 0.0f);

    constexpr int32_t frames = 256;
    std::vector<float> left(frames), right(frames), out_left(frames), out_right(frames);
    const float* input[2]{left.data(), right.data()};
    float* output[2]{out_left.data(), out_right.data()};
    bool received_rendered_audio = false;

    for (int block = 0; block < 180; ++block) {
        for (int32_t i = 0; i < frames; ++i) {
            const float phase = static_cast<float>(block * frames + i);
            left[i] = 0.12f * std::sin(phase * 0.017f) + 0.035f * std::sin(phase * 0.071f);
            right[i] = 0.10f * std::sin(phase * 0.019f) - 0.030f * std::sin(phase * 0.063f);
            out_left[i] = 99.0f;
            out_right[i] = 99.0f;
        }
        effect->process_float(effect, input, output, frames);
        for (int32_t i = 0; i < frames; ++i) {
            if (!std::isfinite(out_left[i]) || !std::isfinite(out_right[i])) {
                effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
                FreeLibrary(module);
                return fail("Current-model output contained non-finite PCM");
            }
            if (std::abs(out_left[i]) > 1.0e-7f || std::abs(out_right[i]) > 1.0e-7f) received_rendered_audio = true;
        }
        Sleep(8);
    }

    if (blocks_fn() == 0 || !received_rendered_audio) {
        effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
        FreeLibrary(module);
        return fail("Current-model worker never returned rendered audio");
    }

    // VST bypass must remain exact even while the Current worker is live.
    effect->control(effect, kVstEffectBypass, 0, 1, nullptr, 0.0f);
    for (int32_t i = 0; i < frames; ++i) {
        left[i] = static_cast<float>(i) / 1000.0f;
        right[i] = -static_cast<float>(i) / 997.0f;
        out_left[i] = 0.0f;
        out_right[i] = 0.0f;
    }
    effect->process_float(effect, input, output, frames);
    if (!equal_bits(left, out_left) || !equal_bits(right, out_right)) {
        effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
        FreeLibrary(module);
        return fail("explicit VST bypass was not bit-exact");
    }
    effect->control(effect, kVstEffectBypass, 0, 0, nullptr, 0.0f);

    // Blocks larger than the prepared host maximum fail safely to direct PCM.
    constexpr int32_t oversized_frames = 300;
    std::vector<float> big_left(oversized_frames), big_right(oversized_frames);
    std::vector<float> big_out_left(oversized_frames), big_out_right(oversized_frames);
    for (int32_t i = 0; i < oversized_frames; ++i) {
        big_left[i] = static_cast<float>(i) / 1000.0f;
        big_right[i] = -static_cast<float>(i) / 997.0f;
    }
    const float* big_input[2]{big_left.data(), big_right.data()};
    float* big_output[2]{big_out_left.data(), big_out_right.data()};
    effect->process_float(effect, big_input, big_output, oversized_frames);
    if (!equal_bits(big_left, big_out_left) || !equal_bits(big_right, big_out_right)) {
        effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
        FreeLibrary(module);
        return fail("oversized-block safety fallback was not passthrough");
    }

    effect->control(effect, kVstEffectProcessEnd, 0, 0, nullptr, 0.0f);
    effect->control(effect, kVstEffectDestroy, 0, 0, nullptr, 0.0f);
    if (ready_fn() != 0 || current_fn() != 0) {
        FreeLibrary(module);
        return fail("backend instance leaked across VST destroy");
    }
    FreeLibrary(module);
    std::cout << "PASS: VST bridge activated Omniphony Current via Rust ABI 0.2 and returned finite rendered stereo\n";
    return 0;
}
