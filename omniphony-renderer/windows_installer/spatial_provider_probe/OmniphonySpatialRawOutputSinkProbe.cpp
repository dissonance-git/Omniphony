#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include "OmniphonySpatialRawOutputSink.h"

#include <cerrno>
#include <climits>
#include <cstdlib>
#include <iomanip>
#include <iostream>

namespace {

class CoInit final {
public:
    CoInit() noexcept : hr_(CoInitializeEx(nullptr, COINIT_MULTITHREADED)) {}
    ~CoInit() {
        if (SUCCEEDED(hr_)) {
            CoUninitialize();
        }
    }
    HRESULT Result() const noexcept { return hr_; }

private:
    HRESULT hr_ = E_FAIL;
};

int Fail(const wchar_t* stage, HRESULT hr) {
    std::wcerr << L"SPATIAL_RAW_OUTPUT_SINK_FAIL stage=" << stage
               << L" hr=0x" << std::hex << std::uppercase
               << static_cast<unsigned long>(hr) << std::dec << L"\n";
    return 1;
}

bool ParsePeriod(const wchar_t* text, std::uint32_t& period) noexcept {
    if (!text || !text[0]) {
        return false;
    }
    errno = 0;
    wchar_t* end = nullptr;
    const unsigned long value = std::wcstoul(text, &end, 10);
    if (errno != 0 || !end || *end != L'\0' || value == 0 || value > UINT32_MAX) {
        return false;
    }
    period = static_cast<std::uint32_t>(value);
    return true;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc < 2 || argc > 3 || !argv[1] || !argv[1][0]) {
        std::wcerr << L"usage: OmniphonySpatialRawOutputSinkProbe.exe "
                   << L"<physical-endpoint-id> [exact-period-frames]\n";
        return 2;
    }

    std::uint32_t requestedPeriodFrames = 0;
    if (argc == 3 && !ParsePeriod(argv[2], requestedPeriodFrames)) {
        std::wcerr << L"SPATIAL_RAW_OUTPUT_SINK_BAD_PERIOD\n";
        return 2;
    }

    CoInit co;
    if (FAILED(co.Result()) && co.Result() != RPC_E_CHANGED_MODE) {
        return Fail(L"CoInitializeEx", co.Result());
    }

    OmniphonySpatialRawOutputSink sink;
    const HRESULT hr = sink.Open(argv[1], requestedPeriodFrames);
    if (FAILED(hr)) {
        return Fail(L"Open", hr);
    }

    if (!sink.IsInitialized() || sink.IsStarted() || !sink.HasRenderClient() ||
        !sink.SampleReadyEvent() || sink.BufferFrames() == 0 ||
        sink.PeriodFrames() == 0 || sink.SampleRateHz() != 48'000) {
        std::wcerr << L"SPATIAL_RAW_OUTPUT_SINK_STATE_INVALID\n";
        return 3;
    }
    if (requestedPeriodFrames != 0 && sink.PeriodFrames() != requestedPeriodFrames) {
        std::wcerr << L"SPATIAL_RAW_OUTPUT_SINK_PERIOD_MISMATCH\n";
        return 3;
    }

    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_OK 1\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_MODE RAW\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_FORMAT FLOAT32_STEREO_48000\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_PERIOD_SOURCE "
               << (requestedPeriodFrames == 0 ? L"ENDPOINT_DEFAULT" : L"EXACT_REQUEST")
               << L"\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_PERIOD_FRAMES "
               << sink.PeriodFrames() << L"\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_BUFFER_FRAMES "
               << sink.BufferFrames() << L"\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_CLOCK_ADAPTER_REQUIRED "
               << (sink.PeriodFrames() == 480 ? 0 : 1) << L"\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_RENDER_CLIENT 1\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_EVENT_HANDLE 1\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_INITIALIZED 1\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_STARTED 0\n";

    sink.Close();
    if (sink.IsInitialized() || sink.IsStarted() || sink.HasRenderClient() ||
        sink.SampleReadyEvent() || sink.BufferFrames() != 0 ||
        sink.PeriodFrames() != 0 || sink.SampleRateHz() != 0) {
        std::wcerr << L"SPATIAL_RAW_OUTPUT_SINK_CLOSE_FAILED\n";
        return 4;
    }

    std::wcout << L"SPATIAL_RAW_OUTPUT_SINK_CLOSE_OK 1\n";
    return 0;
}
