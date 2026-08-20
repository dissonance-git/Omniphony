#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include <iostream>
#include <memory>

#include "OmniphonySpatialRawOutputPump.h"

namespace {

int Fail(const char* stage, HRESULT hr = E_FAIL) {
    std::cerr << "SPATIAL_RAW_OUTPUT_PUMP_SMOKE_FAIL stage=" << stage
              << " hr=0x" << std::hex
              << static_cast<unsigned long>(hr) << std::dec << "\n";
    return 1;
}

} // namespace

int main() {
    auto queue = std::make_shared<OmniphonySpatialStereoQueue>();
    if (!queue->Open(1920)) {
        return Fail("queue-open", E_OUTOFMEMORY);
    }

    OmniphonySpatialRawOutputPump pump;
    if (pump.IsOpen() || pump.IsStarted() || pump.SampleReadyEvent() ||
        pump.BufferFrames() != 0 || pump.PeriodFrames() != 0) {
        return Fail("initial-state");
    }

    // This smoke deliberately never names a physical endpoint and therefore
    // can run in CI without creating an audio stream. It still compiles and
    // links the complete active pump implementation and exercises fail-closed
    // lifecycle behavior before endpoint acquisition.
    const HRESULT nullEndpoint = pump.Open(nullptr, queue);
    if (nullEndpoint != E_INVALIDARG || pump.IsOpen() || pump.IsStarted()) {
        return Fail("null-endpoint", nullEndpoint);
    }

    const HRESULT startWithoutOpen = pump.Start();
    if (startWithoutOpen != E_UNEXPECTED || pump.IsStarted()) {
        return Fail("start-without-open", startWithoutOpen);
    }

    if (FAILED(pump.Stop())) {
        return Fail("idempotent-stop");
    }
    pump.Close();
    if (pump.IsOpen() || pump.IsStarted() || pump.DrainCycles() != 0 ||
        pump.RealFramesWritten() != 0 || pump.SilenceFramesWritten() != 0) {
        return Fail("closed-state");
    }

    std::cout << "SPATIAL_RAW_OUTPUT_PUMP_CONTRACT_OK 1\n";
    std::cout << "SPATIAL_RAW_OUTPUT_PUMP_PHYSICAL_ENDPOINT_OPENED 0\n";
    std::cout << "SPATIAL_RAW_OUTPUT_PUMP_STREAM_STARTED 0\n";
    std::cout << "SPATIAL_RAW_OUTPUT_PUMP_PUBLIC_PROVIDER_GATE_OPENED 0\n";
    return 0;
}
