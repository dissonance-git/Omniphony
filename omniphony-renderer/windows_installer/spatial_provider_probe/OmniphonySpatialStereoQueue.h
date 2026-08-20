#pragma once

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <vector>

// Preallocated single-producer/single-consumer stereo frame queue between the
// fixed Omniphony spatial render quantum and the physical endpoint event clock.
//
// The producer may submit 480-frame Current output blocks while the consumer
// drains any legal endpoint period. Open/Close/Reset are control-thread calls;
// TryWrite and Read are allocation-free and non-blocking after Open.
class OmniphonySpatialStereoQueue final {
public:
    OmniphonySpatialStereoQueue() = default;

    OmniphonySpatialStereoQueue(const OmniphonySpatialStereoQueue&) = delete;
    OmniphonySpatialStereoQueue& operator=(const OmniphonySpatialStereoQueue&) = delete;

    bool Open(std::size_t capacityFrames);
    void Close() noexcept;
    void Reset() noexcept;

    bool IsOpen() const noexcept { return capacityFrames_ != 0; }
    std::size_t CapacityFrames() const noexcept { return capacityFrames_; }
    std::size_t AvailableFrames() const noexcept;
    std::size_t FreeFrames() const noexcept;

    // Writes one complete interleaved-stereo block or rejects it as a whole.
    // Rejection is preferable to blocking the Spatial Audio update path.
    bool TryWrite(const float* stereoInterleaved, std::size_t frames) noexcept;

    // Reads up to frames interleaved-stereo frames. Any unavailable tail is
    // explicitly zero-filled so endpoint output never exposes stale memory.
    // Returns the number of real queued frames copied before zero fill.
    std::size_t Read(float* stereoInterleaved, std::size_t frames) noexcept;

    std::uint64_t DroppedFrames() const noexcept {
        return droppedFrames_.load(std::memory_order_relaxed);
    }
    std::uint64_t UnderrunFrames() const noexcept {
        return underrunFrames_.load(std::memory_order_relaxed);
    }

private:
    std::vector<float> storage_;
    std::size_t capacityFrames_ = 0;
    std::atomic<std::uint64_t> writeFrame_{0};
    std::atomic<std::uint64_t> readFrame_{0};
    std::atomic<std::uint64_t> droppedFrames_{0};
    std::atomic<std::uint64_t> underrunFrames_{0};
};
