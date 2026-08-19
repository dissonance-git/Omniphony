#include "OmniphonySpatialStereoQueue.h"

#include <algorithm>
#include <cstring>
#include <limits>

namespace {

constexpr std::size_t kStereoChannels = 2;

void CopyStereoFrames(
    float* destination,
    const float* source,
    std::size_t frames) noexcept {
    if (frames == 0) {
        return;
    }
    std::memcpy(
        destination,
        source,
        frames * kStereoChannels * sizeof(float));
}

} // namespace

bool OmniphonySpatialStereoQueue::Open(std::size_t capacityFrames) {
    Close();
    if (capacityFrames == 0 ||
        capacityFrames > (std::numeric_limits<std::size_t>::max() / kStereoChannels)) {
        return false;
    }

    try {
        storage_.assign(capacityFrames * kStereoChannels, 0.0f);
    }
    catch (...) {
        storage_.clear();
        return false;
    }

    capacityFrames_ = capacityFrames;
    Reset();
    return true;
}

void OmniphonySpatialStereoQueue::Close() noexcept {
    capacityFrames_ = 0;
    writeFrame_.store(0, std::memory_order_relaxed);
    readFrame_.store(0, std::memory_order_relaxed);
    droppedFrames_.store(0, std::memory_order_relaxed);
    underrunFrames_.store(0, std::memory_order_relaxed);
    storage_.clear();
}

void OmniphonySpatialStereoQueue::Reset() noexcept {
    writeFrame_.store(0, std::memory_order_relaxed);
    readFrame_.store(0, std::memory_order_relaxed);
    droppedFrames_.store(0, std::memory_order_relaxed);
    underrunFrames_.store(0, std::memory_order_relaxed);
    std::fill(storage_.begin(), storage_.end(), 0.0f);
}

std::size_t OmniphonySpatialStereoQueue::AvailableFrames() const noexcept {
    if (!IsOpen()) {
        return 0;
    }
    const std::uint64_t write = writeFrame_.load(std::memory_order_acquire);
    const std::uint64_t read = readFrame_.load(std::memory_order_acquire);
    const std::uint64_t available = write - read;
    return static_cast<std::size_t>(std::min<std::uint64_t>(
        available,
        static_cast<std::uint64_t>(capacityFrames_)));
}

std::size_t OmniphonySpatialStereoQueue::FreeFrames() const noexcept {
    return IsOpen() ? capacityFrames_ - AvailableFrames() : 0;
}

bool OmniphonySpatialStereoQueue::TryWrite(
    const float* stereoInterleaved,
    std::size_t frames) noexcept {
    if (!IsOpen() || !stereoInterleaved || frames == 0 || frames > capacityFrames_) {
        return false;
    }

    const std::uint64_t write = writeFrame_.load(std::memory_order_relaxed);
    const std::uint64_t read = readFrame_.load(std::memory_order_acquire);
    const std::uint64_t used = write - read;
    const std::uint64_t capacity = static_cast<std::uint64_t>(capacityFrames_);
    if (used > capacity || static_cast<std::uint64_t>(frames) > capacity - used) {
        droppedFrames_.fetch_add(
            static_cast<std::uint64_t>(frames),
            std::memory_order_relaxed);
        return false;
    }

    const std::size_t start = static_cast<std::size_t>(write % capacity);
    const std::size_t first = std::min(frames, capacityFrames_ - start);
    CopyStereoFrames(
        storage_.data() + start * kStereoChannels,
        stereoInterleaved,
        first);

    const std::size_t second = frames - first;
    if (second != 0) {
        CopyStereoFrames(
            storage_.data(),
            stereoInterleaved + first * kStereoChannels,
            second);
    }

    writeFrame_.store(
        write + static_cast<std::uint64_t>(frames),
        std::memory_order_release);
    return true;
}

std::size_t OmniphonySpatialStereoQueue::Read(
    float* stereoInterleaved,
    std::size_t frames) noexcept {
    if (!IsOpen() || !stereoInterleaved || frames == 0) {
        return 0;
    }

    const std::uint64_t read = readFrame_.load(std::memory_order_relaxed);
    const std::uint64_t write = writeFrame_.load(std::memory_order_acquire);
    const std::uint64_t capacity = static_cast<std::uint64_t>(capacityFrames_);
    const std::uint64_t queued = std::min<std::uint64_t>(write - read, capacity);
    const std::size_t available = static_cast<std::size_t>(
        std::min<std::uint64_t>(queued, static_cast<std::uint64_t>(frames)));

    if (available != 0) {
        const std::size_t start = static_cast<std::size_t>(read % capacity);
        const std::size_t first = std::min(available, capacityFrames_ - start);
        CopyStereoFrames(
            stereoInterleaved,
            storage_.data() + start * kStereoChannels,
            first);

        const std::size_t second = available - first;
        if (second != 0) {
            CopyStereoFrames(
                stereoInterleaved + first * kStereoChannels,
                storage_.data(),
                second);
        }
    }

    if (available < frames) {
        std::fill(
            stereoInterleaved + available * kStereoChannels,
            stereoInterleaved + frames * kStereoChannels,
            0.0f);
        underrunFrames_.fetch_add(
            static_cast<std::uint64_t>(frames - available),
            std::memory_order_relaxed);
    }

    readFrame_.store(
        read + static_cast<std::uint64_t>(available),
        std::memory_order_release);
    return available;
}
