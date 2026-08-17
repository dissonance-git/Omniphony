#include <windows.h>

#include "omniphony_realtime.h"

#include <array>
#include <cstdint>
#include <cstring>
#include <iostream>

namespace {
using AbiFn = uint32_t (*)();
using CreateFn = OmniphonyRealtimeProcessor* (*)(const OmniphonyRealtimeConfig*);
using DestroyFn = void (*)(OmniphonyRealtimeProcessor*);
using SetModeFn = int32_t (*)(OmniphonyRealtimeProcessor*, uint32_t);
using ModeFn = uint32_t (*)(const OmniphonyRealtimeProcessor*);
using ProcessFn = int32_t (*)(OmniphonyRealtimeProcessor*, const float*, float*, size_t);
using BlocksFn = uint64_t (*)(const OmniphonyRealtimeProcessor*);
using LatencyFramesFn = size_t (*)(const OmniphonyRealtimeProcessor*);

template <typename T>
T Resolve(HMODULE module, const char* name) {
    return reinterpret_cast<T>(GetProcAddress(module, name));
}
} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::wcerr << L"usage: OmniphonyRealtimeSmoke.exe <omniphony_realtime.dll>" << std::endl;
        return 2;
    }

    HMODULE module = LoadLibraryW(argv[1]);
    if (!module) {
        std::wcerr << L"REALTIME_LOAD_FAILED\t" << GetLastError() << std::endl;
        return 3;
    }

    const auto abiMajor = Resolve<AbiFn>(module, "omniphony_realtime_abi_major");
    const auto abiMinor = Resolve<AbiFn>(module, "omniphony_realtime_abi_minor");
    const auto create = Resolve<CreateFn>(module, "omniphony_realtime_create");
    const auto destroy = Resolve<DestroyFn>(module, "omniphony_realtime_destroy");
    const auto setMode = Resolve<SetModeFn>(module, "omniphony_realtime_set_mode");
    const auto mode = Resolve<ModeFn>(module, "omniphony_realtime_mode");
    const auto process = Resolve<ProcessFn>(module, "omniphony_realtime_process_f32");
    const auto blocks = Resolve<BlocksFn>(module, "omniphony_realtime_processed_blocks");
    const auto latencyFrames = Resolve<LatencyFramesFn>(module, "omniphony_realtime_latency_frames");

    if (!abiMajor || !abiMinor || !create || !destroy || !setMode || !mode ||
        !process || !blocks || !latencyFrames) {
        std::wcerr << L"REALTIME_EXPORTS_MISSING" << std::endl;
        FreeLibrary(module);
        return 4;
    }
    if (abiMajor() != OMNIPHONY_REALTIME_ABI_MAJOR ||
        abiMinor() < OMNIPHONY_REALTIME_ABI_MINOR) {
        std::wcerr << L"REALTIME_ABI_MISMATCH\t" << abiMajor() << L"." << abiMinor() << std::endl;
        FreeLibrary(module);
        return 5;
    }

    const OmniphonyRealtimeConfig config{48000u, 2u};
    OmniphonyRealtimeProcessor* processor = create(&config);
    if (!processor) {
        std::wcerr << L"REALTIME_CREATE_FAILED" << std::endl;
        FreeLibrary(module);
        return 6;
    }

    int result = 0;
    if (setMode(processor, OMNIPHONY_REALTIME_MODE_IDENTITY) != 0 ||
        mode(processor) != OMNIPHONY_REALTIME_MODE_IDENTITY ||
        latencyFrames(processor) != 0u) {
        std::wcerr << L"REALTIME_IDENTITY_MODE_FAILED" << std::endl;
        result = 7;
    } else {
        const std::array<float, 8> input = {0.0f, -0.25f, 0.5f, 1.0f, -1.0f, 0.125f, -0.75f, 0.875f};
        std::array<float, 8> output = {};
        if (process(processor, input.data(), output.data(), 4u) != 0 ||
            std::memcmp(input.data(), output.data(), sizeof(input)) != 0) {
            std::wcerr << L"REALTIME_IDENTITY_PROCESS_FAILED" << std::endl;
            result = 8;
        } else if (blocks(processor) != 0u) {
            std::wcerr << L"REALTIME_IDENTITY_BLOCK_COUNTER_CHANGED" << std::endl;
            result = 9;
        } else if (setMode(processor, OMNIPHONY_REALTIME_MODE_CURRENT) != 0 ||
                   mode(processor) != OMNIPHONY_REALTIME_MODE_CURRENT) {
            std::wcerr << L"REALTIME_CURRENT_MODE_FAILED" << std::endl;
            result = 10;
        } else if (latencyFrames(processor) != 1920u) {
            std::wcerr << L"REALTIME_CURRENT_LATENCY_FAILED\tFRAMES="
                       << latencyFrames(processor) << std::endl;
            result = 11;
        } else {
            std::wcout << L"REALTIME_DLL_OK\tABI=" << abiMajor() << L"." << abiMinor()
                       << L"\tIDENTITY_BIT_EXACT=1\tCURRENT_INIT=1\tCURRENT_LATENCY_FRAMES=1920"
                       << std::endl;
        }
    }

    destroy(processor);
    FreeLibrary(module);
    return result;
}
