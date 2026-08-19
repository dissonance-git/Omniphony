#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <spatialaudioclient.h>

#include <array>
#include <cstddef>
#include <cstdint>

#include "omniphony_realtime.h"

// One ordered definition of the Windows static-object vocabulary used by the
// provider-side transport. The order is also the descriptor / planar PCM order
// handed to omniphony_realtime.dll for the lifetime of a static stream.
struct OmniphonySpatialStaticRoleDefinition {
    AudioObjectType audio_object_type;
    std::uint32_t omniphony_role;
    float x_right_m;
    float y_up_m;
    float z_back_m;
};

inline constexpr float kOmniphonySpatialDiagonal = 0.70710678f;

inline constexpr std::array<OmniphonySpatialStaticRoleDefinition, 17>
    kOmniphonySpatialStaticRoles = {{
        {AudioObjectType_FrontLeft,
         OMNIPHONY_SPATIAL_STATIC_FRONT_LEFT,
         -kOmniphonySpatialDiagonal, 0.0f, -kOmniphonySpatialDiagonal},
        {AudioObjectType_FrontRight,
         OMNIPHONY_SPATIAL_STATIC_FRONT_RIGHT,
         kOmniphonySpatialDiagonal, 0.0f, -kOmniphonySpatialDiagonal},
        {AudioObjectType_FrontCenter,
         OMNIPHONY_SPATIAL_STATIC_FRONT_CENTER,
         0.0f, 0.0f, -1.0f},
        // Keep the provider-facing Windows capability coordinate stable. The
        // Omniphony realtime renderer still treats LFE as non-directional and
        // ignores descriptor position for this semantic role.
        {AudioObjectType_LowFrequency,
         OMNIPHONY_SPATIAL_STATIC_LOW_FREQUENCY,
         0.0f, 0.0f, -1.0f},
        {AudioObjectType_SideLeft,
         OMNIPHONY_SPATIAL_STATIC_SIDE_LEFT,
         -1.0f, 0.0f, 0.0f},
        {AudioObjectType_SideRight,
         OMNIPHONY_SPATIAL_STATIC_SIDE_RIGHT,
         1.0f, 0.0f, 0.0f},
        {AudioObjectType_BackLeft,
         OMNIPHONY_SPATIAL_STATIC_BACK_LEFT,
         -kOmniphonySpatialDiagonal, 0.0f, kOmniphonySpatialDiagonal},
        {AudioObjectType_BackRight,
         OMNIPHONY_SPATIAL_STATIC_BACK_RIGHT,
         kOmniphonySpatialDiagonal, 0.0f, kOmniphonySpatialDiagonal},
        {AudioObjectType_BackCenter,
         OMNIPHONY_SPATIAL_STATIC_BACK_CENTER,
         0.0f, 0.0f, 1.0f},
        {AudioObjectType_TopFrontLeft,
         OMNIPHONY_SPATIAL_STATIC_TOP_FRONT_LEFT,
         -0.5f, kOmniphonySpatialDiagonal, -0.5f},
        {AudioObjectType_TopFrontRight,
         OMNIPHONY_SPATIAL_STATIC_TOP_FRONT_RIGHT,
         0.5f, kOmniphonySpatialDiagonal, -0.5f},
        {AudioObjectType_TopBackLeft,
         OMNIPHONY_SPATIAL_STATIC_TOP_BACK_LEFT,
         -0.5f, kOmniphonySpatialDiagonal, 0.5f},
        {AudioObjectType_TopBackRight,
         OMNIPHONY_SPATIAL_STATIC_TOP_BACK_RIGHT,
         0.5f, kOmniphonySpatialDiagonal, 0.5f},
        {AudioObjectType_BottomFrontLeft,
         OMNIPHONY_SPATIAL_STATIC_BOTTOM_FRONT_LEFT,
         -0.5f, -kOmniphonySpatialDiagonal, -0.5f},
        {AudioObjectType_BottomFrontRight,
         OMNIPHONY_SPATIAL_STATIC_BOTTOM_FRONT_RIGHT,
         0.5f, -kOmniphonySpatialDiagonal, -0.5f},
        {AudioObjectType_BottomBackLeft,
         OMNIPHONY_SPATIAL_STATIC_BOTTOM_BACK_LEFT,
         -0.5f, -kOmniphonySpatialDiagonal, 0.5f},
        {AudioObjectType_BottomBackRight,
         OMNIPHONY_SPATIAL_STATIC_BOTTOM_BACK_RIGHT,
         0.5f, -kOmniphonySpatialDiagonal, 0.5f},
    }};

inline constexpr std::uint32_t OmniphonySpatialObjectBits(AudioObjectType type) noexcept {
    return static_cast<std::uint32_t>(type);
}

inline constexpr AudioObjectType OmniphonyCanonicalStaticMask() noexcept {
    std::uint32_t mask = 0;
    for (const auto& role : kOmniphonySpatialStaticRoles) {
        mask |= OmniphonySpatialObjectBits(role.audio_object_type);
    }
    return static_cast<AudioObjectType>(mask);
}

inline constexpr bool OmniphonyIsSingleStaticObjectType(AudioObjectType type) noexcept {
    const auto bits = OmniphonySpatialObjectBits(type);
    return bits != 0 && (bits & (bits - 1)) == 0;
}

inline constexpr const OmniphonySpatialStaticRoleDefinition*
FindOmniphonySpatialStaticRole(AudioObjectType type) noexcept {
    for (const auto& role : kOmniphonySpatialStaticRoles) {
        if (role.audio_object_type == type) {
            return &role;
        }
    }
    return nullptr;
}

inline constexpr std::size_t OmniphonyStaticRoleSlot(
    AudioObjectType mask,
    AudioObjectType type) noexcept {
    std::size_t slot = 0;
    for (const auto& role : kOmniphonySpatialStaticRoles) {
        const auto roleBits = OmniphonySpatialObjectBits(role.audio_object_type);
        if ((OmniphonySpatialObjectBits(mask) & roleBits) == 0) {
            continue;
        }
        if (role.audio_object_type == type) {
            return slot;
        }
        ++slot;
    }
    return static_cast<std::size_t>(-1);
}

inline constexpr std::size_t OmniphonyStaticRoleCount(AudioObjectType mask) noexcept {
    std::size_t count = 0;
    for (const auto& role : kOmniphonySpatialStaticRoles) {
        if ((OmniphonySpatialObjectBits(mask) &
             OmniphonySpatialObjectBits(role.audio_object_type)) != 0) {
            ++count;
        }
    }
    return count;
}
