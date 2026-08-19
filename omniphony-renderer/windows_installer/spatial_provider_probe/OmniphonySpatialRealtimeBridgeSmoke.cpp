#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <iostream>
#include <vector>

#include "OmniphonySpatialRealtimeBridge.h"

namespace {

int Fail(const wchar_t* stage, HRESULT hr) {
    std::wcerr << L"SPATIAL_REALTIME_BRIDGE_SMOKE_FAIL stage=" << stage
               << L" hr=0x" << std::hex << std::uppercase
               << static_cast<unsigned long>(hr) << std::dec << L"\n";
    return 1;
}

bool AllFinite(const std::vector<float>& samples) {
    return std::all_of(samples.begin(), samples.end(), [](float sample) {
        return std::isfinite(sample);
    });
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::wcerr << L"usage: OmniphonySpatialRealtimeBridgeSmoke C:\\absolute\\path\\omniphony_realtime.dll\n";
        return 2;
    }

    constexpr std::uint32_t sampleRate = 48'000;
    constexpr std::uint32_t frames = 480;
    constexpr std::uint32_t objectCount = 2;

    const OmniphonySpatialStaticObjectDescriptor descriptors[objectCount] = {
        {
            OMNIPHONY_SPATIAL_STATIC_FRONT_LEFT,
            -0.70710678f,
            0.0f,
            -0.70710678f,
        },
        {
            OMNIPHONY_SPATIAL_STATIC_TOP_FRONT_LEFT,
            -0.5f,
            0.70710678f,
            -0.5f,
        },
    };

    OmniphonySpatialRealtimeBridge bridge;
    HRESULT hr = bridge.Open(
        argv[1],
        sampleRate,
        frames,
        descriptors,
        objectCount);
    if (FAILED(hr) || !bridge.IsOpen()) {
        return Fail(L"Open", FAILED(hr) ? hr : E_FAIL);
    }

    if (bridge.LatencyFrames() == 0) {
        return Fail(L"LatencyFrames", E_FAIL);
    }

    std::vector<float> planar(static_cast<std::size_t>(frames) * objectCount);
    std::vector<float> stereo(static_cast<std::size_t>(frames) * 2);
    float peak = 0.0f;

    // Run enough 10 ms quanta to cover the worker/fallback delay and prove the
    // dynamically loaded static-object ABI is consuming complete planar blocks.
    for (std::uint32_t quantum = 0; quantum < 16; ++quantum) {
        for (std::uint32_t frame = 0; frame < frames; ++frame) {
            const std::uint32_t phase = (quantum * frames + frame) % 64;
            planar[frame] = phase < 32 ? 0.05f : -0.05f;
            planar[frames + frame] = phase < 16 ? 0.04f : -0.04f;
        }
        std::fill(stereo.begin(), stereo.end(), 0.0f);

        hr = bridge.Process(planar.data(), stereo.data(), frames);
        if (FAILED(hr)) {
            return Fail(L"Process", hr);
        }
        if (!AllFinite(stereo)) {
            return Fail(L"finite-output", E_FAIL);
        }
        for (float sample : stereo) {
            peak = std::max(peak, std::abs(sample));
        }

        // This is a registry-free diagnostic executable, not an audio callback.
        // Yielding here gives the dedicated Current worker deterministic room to
        // consume the submitted quantum without turning the ABI into a busy loop.
        Sleep(10);
    }

    if (bridge.ProcessedBlocks() == 0) {
        return Fail(L"ProcessedBlocks", E_FAIL);
    }
    if (!(peak > 0.0f) || !std::isfinite(peak)) {
        return Fail(L"nonzero-output", E_FAIL);
    }

    std::wcout << L"SPATIAL_REALTIME_BRIDGE_OK 1\n";
    std::wcout << L"SPATIAL_REALTIME_BRIDGE_OBJECTS " << objectCount << L"\n";
    std::wcout << L"SPATIAL_REALTIME_BRIDGE_FRAMES " << frames << L"\n";
    std::wcout << L"SPATIAL_REALTIME_BRIDGE_LATENCY_FRAMES "
               << bridge.LatencyFrames() << L"\n";
    std::wcout << L"SPATIAL_REALTIME_BRIDGE_PROCESSED_BLOCKS "
               << bridge.ProcessedBlocks() << L"\n";
    std::wcout << L"SPATIAL_REALTIME_BRIDGE_OUTPUT_PEAK " << peak << L"\n";
    return 0;
}
