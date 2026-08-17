#include <windows.h>

#include <atomic>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <new>
#include <string>
#include <vector>

#include "vst2_minimal.h"

namespace {

constexpr uint32_t kChannels = 2;
constexpr uint32_t kExpectedAbiMajor = 0;
constexpr uint32_t kMinimumAbiMinor = 1;
constexpr int32_t kPluginVersion = 1000;
constexpr int32_t kUniqueId =
    static_cast<int32_t>(OMNI_VST_FOURCC('O', 'm', 'I', 'd'));

HMODULE g_this_module = nullptr;
std::atomic<uint32_t> g_ready_instances{0};
std::atomic<uint32_t> g_last_abi_minor{0};

struct OmniphonyRealtimeProcessor;
struct OmniphonyRealtimeConfig {
    uint32_t sample_rate_hz;
    uint32_t channels;
};

using AbiFn = uint32_t(*)();
using CreateFn = OmniphonyRealtimeProcessor*(*)(const OmniphonyRealtimeConfig*);
using DestroyFn = void(*)(OmniphonyRealtimeProcessor*);
using ResetFn = int32_t(*)(OmniphonyRealtimeProcessor*);
using ProcessFn = int32_t(*)(
    OmniphonyRealtimeProcessor*, const float*, float*, size_t);

struct Backend {
    HMODULE module = nullptr;
    AbiFn abi_major = nullptr;
    AbiFn abi_minor = nullptr;
    CreateFn create = nullptr;
    DestroyFn destroy = nullptr;
    ResetFn reset = nullptr;
    ProcessFn process = nullptr;
};

struct State {
    Backend backend;
    OmniphonyRealtimeProcessor* processor = nullptr;
    uint32_t sample_rate_hz = 44100;
    int32_t block_size = 1024;
    std::vector<float> interleaved;
    bool ready_counted = false;
    bool bypass = false;
};

void copy_text(void* target, size_t capacity, const char* text) {
    if (target == nullptr || capacity == 0) {
        return;
    }
    char* out = static_cast<char*>(target);
    strncpy_s(out, capacity, text, _TRUNCATE);
}

void set_ready(State* state, bool ready) {
    if (state == nullptr || state->ready_counted == ready) {
        return;
    }
    if (ready) {
        g_ready_instances.fetch_add(1, std::memory_order_relaxed);
    } else {
        g_ready_instances.fetch_sub(1, std::memory_order_relaxed);
    }
    state->ready_counted = ready;
}

void destroy_processor(State* state) {
    if (state == nullptr) {
        return;
    }
    set_ready(state, false);
    if (state->processor != nullptr && state->backend.destroy != nullptr) {
        state->backend.destroy(state->processor);
    }
    state->processor = nullptr;
}

void unload_backend(State* state) {
    if (state == nullptr) {
        return;
    }
    destroy_processor(state);
    if (state->backend.module != nullptr) {
        FreeLibrary(state->backend.module);
    }
    state->backend = {};
}

std::wstring sibling_path(const wchar_t* filename) {
    wchar_t path[32768]{};
    DWORD len = GetModuleFileNameW(
        g_this_module, path, static_cast<DWORD>(_countof(path)));
    if (len == 0 || len >= _countof(path)) {
        return {};
    }
    std::wstring result(path, len);
    const size_t slash = result.find_last_of(L"\\/");
    if (slash == std::wstring::npos) {
        return {};
    }
    result.resize(slash + 1);
    result.append(filename);
    return result;
}

template <typename T>
T symbol(HMODULE module, const char* name) {
    return reinterpret_cast<T>(GetProcAddress(module, name));
}

bool load_backend(State* state) {
    if (state == nullptr) {
        return false;
    }
    if (state->backend.module != nullptr) {
        return true;
    }

    const std::wstring dll_path = sibling_path(L"omniphony_realtime.dll");
    if (dll_path.empty()) {
        return false;
    }

    Backend backend;
    backend.module = LoadLibraryW(dll_path.c_str());
    if (backend.module == nullptr) {
        return false;
    }

    backend.abi_major = symbol<AbiFn>(
        backend.module, "omniphony_realtime_abi_major");
    backend.abi_minor = symbol<AbiFn>(
        backend.module, "omniphony_realtime_abi_minor");
    backend.create = symbol<CreateFn>(
        backend.module, "omniphony_realtime_create");
    backend.destroy = symbol<DestroyFn>(
        backend.module, "omniphony_realtime_destroy");
    backend.reset = symbol<ResetFn>(
        backend.module, "omniphony_realtime_reset");
    backend.process = symbol<ProcessFn>(
        backend.module, "omniphony_realtime_process_f32");

    if (backend.abi_major == nullptr || backend.abi_minor == nullptr ||
        backend.create == nullptr || backend.destroy == nullptr ||
        backend.reset == nullptr || backend.process == nullptr) {
        FreeLibrary(backend.module);
        return false;
    }

    const uint32_t abi_major = backend.abi_major();
    const uint32_t abi_minor = backend.abi_minor();
    if (abi_major != kExpectedAbiMajor || abi_minor < kMinimumAbiMinor) {
        FreeLibrary(backend.module);
        return false;
    }

    g_last_abi_minor.store(abi_minor, std::memory_order_relaxed);
    state->backend = backend;
    return true;
}

bool resize_buffers(State* state, int32_t block_size) {
    if (state == nullptr || block_size <= 0) {
        return false;
    }
    try {
        state->interleaved.resize(
            static_cast<size_t>(block_size) * kChannels);
        state->block_size = block_size;
        return true;
    } catch (...) {
        state->interleaved.clear();
        state->block_size = 0;
        return false;
    }
}

bool rebuild_processor(State* state) {
    if (state == nullptr) {
        return false;
    }

    destroy_processor(state);
    if (!load_backend(state)) {
        return false;
    }
    if (state->sample_rate_hz < 8000 || state->sample_rate_hz > 384000) {
        return false;
    }

    const OmniphonyRealtimeConfig config{
        state->sample_rate_hz,
        kChannels,
    };
    state->processor = state->backend.create(&config);
    const bool ready = state->processor != nullptr;
    set_ready(state, ready);
    return ready;
}

State* state_from(vst_effect_t* effect) {
    return effect == nullptr
        ? nullptr
        : static_cast<State*>(effect->effect_internal);
}

void passthrough(
    const float* const* inputs,
    float** outputs,
    int32_t frames) {
    if (inputs == nullptr || outputs == nullptr || frames <= 0) {
        return;
    }
    const size_t bytes = static_cast<size_t>(frames) * sizeof(float);
    for (uint32_t channel = 0; channel < kChannels; ++channel) {
        if (inputs[channel] == nullptr || outputs[channel] == nullptr) {
            continue;
        }
        if (inputs[channel] != outputs[channel]) {
            std::memmove(outputs[channel], inputs[channel], bytes);
        }
    }
}

void __cdecl process_float(
    vst_effect_t* effect,
    const float* const* inputs,
    float** outputs,
    int32_t frames) {
    State* state = state_from(effect);
    if (state == nullptr || inputs == nullptr || outputs == nullptr ||
        frames <= 0) {
        return;
    }

    const size_t needed = static_cast<size_t>(frames) * kChannels;
    if (state->bypass || state->processor == nullptr ||
        state->backend.process == nullptr ||
        state->block_size < frames ||
        state->interleaved.size() < needed ||
        inputs[0] == nullptr || inputs[1] == nullptr ||
        outputs[0] == nullptr || outputs[1] == nullptr) {
        passthrough(inputs, outputs, frames);
        return;
    }

    float* interleaved = state->interleaved.data();
    for (int32_t frame = 0; frame < frames; ++frame) {
        const size_t base = static_cast<size_t>(frame) * kChannels;
        interleaved[base] = inputs[0][frame];
        interleaved[base + 1] = inputs[1][frame];
    }

    if (state->backend.process(
            state->processor,
            interleaved,
            interleaved,
            static_cast<size_t>(frames)) != 0) {
        passthrough(inputs, outputs, frames);
        return;
    }

    for (int32_t frame = 0; frame < frames; ++frame) {
        const size_t base = static_cast<size_t>(frame) * kChannels;
        outputs[0][frame] = interleaved[base];
        outputs[1][frame] = interleaved[base + 1];
    }
}

void __cdecl set_parameter(vst_effect_t*, uint32_t, float) {}
float __cdecl get_parameter(vst_effect_t*, uint32_t) { return 0.0f; }

intptr_t __cdecl control(
    vst_effect_t* effect,
    int32_t opcode,
    int32_t,
    intptr_t value,
    void* ptr,
    float opt) {
    State* state = state_from(effect);

    switch (opcode) {
    case kVstEffectInitialize:
        if (state != nullptr) {
            resize_buffers(state, state->block_size);
            rebuild_processor(state);
        }
        return 0;

    case kVstEffectDestroy:
        if (state != nullptr) {
            unload_backend(state);
            delete state;
        }
        delete effect;
        return 0;

    case kVstEffectSetSampleRate:
        if (state != nullptr && std::isfinite(opt) && opt > 0.0f) {
            state->sample_rate_hz =
                static_cast<uint32_t>(std::lround(opt));
            rebuild_processor(state);
        }
        return 0;

    case kVstEffectSetBlockSize:
        if (state != nullptr) {
            resize_buffers(state, static_cast<int32_t>(value));
        }
        return 0;

    case kVstEffectSuspend:
        if (state != nullptr && value != 0 &&
            state->processor != nullptr && state->backend.reset != nullptr) {
            state->backend.reset(state->processor);
        }
        return 0;

    case kVstEffectCategory:
        return kVstEffectCategorySpatial;

    case kVstEffectBypass:
        if (state != nullptr) {
            state->bypass = value != 0;
        }
        return 1;

    case kVstEffectName:
        copy_text(ptr, 32, "Omniphony Identity Bridge");
        return 1;

    case kVstEffectVendorName:
        copy_text(ptr, 64, "Omniphony");
        return 1;

    case kVstEffectProductName:
        copy_text(ptr, 64, "Omniphony Personal Bootstrap");
        return 1;

    case kVstEffectVendorVersion:
        return kPluginVersion;

    case kVstEffectSupports:
        if (ptr != nullptr && std::strcmp(
                static_cast<const char*>(ptr), "bypass") == 0) {
            return 1;
        }
        return 0;

    case kVstEffectTailSamples:
        return 1;

    case kVstEffectVstVersion:
        return kVstVersion2400;

    case kVstEffectProcessBegin:
        if (state != nullptr && state->processor != nullptr &&
            state->backend.reset != nullptr) {
            state->backend.reset(state->processor);
        }
        return 1;

    case kVstEffectProcessEnd:
        return 1;

    default:
        return 0;
    }
}

}  // namespace

extern "C" __declspec(dllexport) vst_effect_t* __cdecl
VSTPluginMain(vst_host_callback_t) {
    auto* effect = new (std::nothrow) vst_effect_t{};
    auto* state = new (std::nothrow) State{};
    if (effect == nullptr || state == nullptr) {
        delete state;
        delete effect;
        return nullptr;
    }

    effect->magic_number = kVstMagic;
    effect->control = control;
    effect->process = process_float;
    effect->set_parameter = set_parameter;
    effect->get_parameter = get_parameter;
    effect->num_programs = 0;
    effect->num_params = 0;
    effect->num_inputs = static_cast<int32_t>(kChannels);
    effect->num_outputs = static_cast<int32_t>(kChannels);
    effect->flags = kVstEffectFlagSupportsFloat;
    effect->delay = 0;
    effect->input_output_ratio = 1.0f;
    effect->effect_internal = state;
    effect->unique_id = kUniqueId;
    effect->version = kPluginVersion;
    effect->process_float = process_float;
    effect->process_double = nullptr;
    return effect;
}

extern "C" __declspec(dllexport) uint32_t
omniphony_vst_bridge_backend_ready_instances() {
    return g_ready_instances.load(std::memory_order_relaxed);
}

extern "C" __declspec(dllexport) uint32_t
omniphony_vst_bridge_backend_abi_minor() {
    return g_last_abi_minor.load(std::memory_order_relaxed);
}

BOOL APIENTRY DllMain(HMODULE module, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        g_this_module = module;
        DisableThreadLibraryCalls(module);
    }
    return TRUE;
}
