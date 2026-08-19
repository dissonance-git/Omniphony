#include "OmniphonySpatialStereoQueue.h"

#include <cstddef>
#include <cstdint>
#include <iostream>
#include <vector>

namespace {

constexpr std::size_t kChannels = 2;

std::vector<float> MakeStereo(std::size_t firstFrame, std::size_t frames) {
    std::vector<float> result(frames * kChannels, 0.0f);
    for (std::size_t frame = 0; frame < frames; ++frame) {
        const float value = static_cast<float>(firstFrame + frame + 1);
        result[frame * kChannels] = value;
        result[frame * kChannels + 1] = -value;
    }
    return result;
}

bool CheckStereo(
    const std::vector<float>& buffer,
    std::size_t firstFrame,
    std::size_t frames) {
    if (buffer.size() != frames * kChannels) {
        return false;
    }
    for (std::size_t frame = 0; frame < frames; ++frame) {
        const float value = static_cast<float>(firstFrame + frame + 1);
        if (buffer[frame * kChannels] != value ||
            buffer[frame * kChannels + 1] != -value) {
            return false;
        }
    }
    return true;
}

bool CheckSilence(const std::vector<float>& buffer) {
    for (const float sample : buffer) {
        if (sample != 0.0f) {
            return false;
        }
    }
    return true;
}

int Fail(const char* stage) {
    std::cerr << "SPATIAL_STEREO_QUEUE_FAIL stage=" << stage << "\n";
    return 1;
}

} // namespace

int main() {
    OmniphonySpatialStereoQueue queue;
    if (!queue.Open(960) || queue.CapacityFrames() != 960 ||
        queue.AvailableFrames() != 0 || queue.FreeFrames() != 960) {
        return Fail("open");
    }

    const auto firstQuantum = MakeStereo(0, 480);
    if (!queue.TryWrite(firstQuantum.data(), 480) ||
        queue.AvailableFrames() != 480) {
        return Fail("write-480");
    }

    // Exercise a deliberately different endpoint cadence. The producer remains
    // fixed at 480 frames while the consumer drains 128 + 224 + 128 frames.
    std::vector<float> read128(128 * kChannels, 0.0f);
    if (queue.Read(read128.data(), 128) != 128 || !CheckStereo(read128, 0, 128)) {
        return Fail("read-128");
    }

    std::vector<float> read224(224 * kChannels, 0.0f);
    if (queue.Read(read224.data(), 224) != 224 || !CheckStereo(read224, 128, 224)) {
        return Fail("read-224");
    }

    std::vector<float> readFinal128(128 * kChannels, 0.0f);
    if (queue.Read(readFinal128.data(), 128) != 128 ||
        !CheckStereo(readFinal128, 352, 128) || queue.AvailableFrames() != 0) {
        return Fail("read-final-128");
    }

    std::vector<float> underrun(64 * kChannels, 1.0f);
    if (queue.Read(underrun.data(), 64) != 0 || !CheckSilence(underrun) ||
        queue.UnderrunFrames() != 64) {
        return Fail("underrun-zero-fill");
    }

    // Force a wrap while preserving exact sample order.
    queue.Reset();
    const auto quantumA = MakeStereo(0, 480);
    const auto quantumB = MakeStereo(480, 480);
    const auto quantumC = MakeStereo(960, 480);
    if (!queue.TryWrite(quantumA.data(), 480) ||
        !queue.TryWrite(quantumB.data(), 480)) {
        return Fail("wrap-fill");
    }

    std::vector<float> consume600(600 * kChannels, 0.0f);
    if (queue.Read(consume600.data(), 600) != 600 ||
        !CheckStereo(consume600, 0, 600)) {
        return Fail("wrap-preread");
    }
    if (!queue.TryWrite(quantumC.data(), 480)) {
        return Fail("wrap-write");
    }

    std::vector<float> consume360(360 * kChannels, 0.0f);
    if (queue.Read(consume360.data(), 360) != 360 ||
        !CheckStereo(consume360, 600, 360)) {
        return Fail("wrap-tail-b");
    }

    std::vector<float> consume480(480 * kChannels, 0.0f);
    if (queue.Read(consume480.data(), 480) != 480 ||
        !CheckStereo(consume480, 960, 480) || queue.AvailableFrames() != 0) {
        return Fail("wrap-c");
    }

    // Overflow is whole-block rejection. It never blocks and never overwrites
    // unread audio behind the consumer.
    queue.Reset();
    if (!queue.TryWrite(quantumA.data(), 480) ||
        !queue.TryWrite(quantumB.data(), 480) ||
        queue.TryWrite(quantumC.data(), 480) ||
        queue.DroppedFrames() != 480 || queue.AvailableFrames() != 960) {
        return Fail("overflow-reject");
    }

    queue.Close();
    if (queue.IsOpen() || queue.CapacityFrames() != 0 ||
        queue.AvailableFrames() != 0 || queue.FreeFrames() != 0) {
        return Fail("close");
    }

    std::cout << "SPATIAL_STEREO_QUEUE_OK 1\n";
    std::cout << "SPATIAL_STEREO_QUEUE_PRODUCER_QUANTUM 480\n";
    std::cout << "SPATIAL_STEREO_QUEUE_VARIABLE_CONSUMER_PERIODS 1\n";
    std::cout << "SPATIAL_STEREO_QUEUE_ZERO_FILL_UNDERRUN 1\n";
    std::cout << "SPATIAL_STEREO_QUEUE_NONBLOCKING_OVERFLOW_REJECT 1\n";
    return 0;
}
