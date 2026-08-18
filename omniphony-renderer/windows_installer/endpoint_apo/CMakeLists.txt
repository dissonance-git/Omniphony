cmake_minimum_required(VERSION 3.20)
cmake_policy(SET CMP0091 NEW)
project(OmniphonyEndpointApo LANGUAGES CXX)

if(NOT WIN32)
    message(FATAL_ERROR "The Omniphony endpoint APO is Windows-only.")
endif()

# Keep the shipped APO/helpers independent of the Visual C++ redistributable.
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
set(OMNIPHONY_REALTIME_INCLUDE "${CMAKE_CURRENT_SOURCE_DIR}/../../realtime_ffi/include")

# Proven post-mix stereo endpoint APO. Retained as a rollback floor while the
# pre-mix stream APO earns field confidence on real Windows endpoints.
add_library(OmniphonyAPO SHARED
    OmniphonyAPO.cpp
    OmniphonyAPO.def
)
target_compile_features(OmniphonyAPO PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyAPO PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyAPO PRIVATE /W4 /WX /permissive-)
target_include_directories(OmniphonyAPO PRIVATE "${OMNIPHONY_REALTIME_INCLUDE}")
target_link_libraries(OmniphonyAPO PRIVATE ole32 uuid advapi32 audioeng AudioBaseProcessingObjectV140 legacy_stdio_definitions audiomediatypecrt)
target_link_options(OmniphonyAPO PRIVATE /MANIFEST:NO)
set_target_properties(OmniphonyAPO PROPERTIES PREFIX "" OUTPUT_NAME "OmniphonyAPO")

# Pre-mix SFX. Stereo streams use the proven Current PCM ABI; authored Windows
# multichannel beds use the native-bed ABI and emerge as binaural stereo before
# the endpoint mixer. Samples-per-frame is intentionally allowed to differ.
add_library(OmniphonyStreamAPO SHARED
    OmniphonyStreamAPO.cpp
    OmniphonyStreamAPO.def
)
target_compile_features(OmniphonyStreamAPO PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyStreamAPO PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyStreamAPO PRIVATE /W4 /WX /permissive-)
target_include_directories(OmniphonyStreamAPO PRIVATE "${OMNIPHONY_REALTIME_INCLUDE}")
target_link_libraries(OmniphonyStreamAPO PRIVATE ole32 uuid advapi32 audioeng AudioBaseProcessingObjectV140 legacy_stdio_definitions audiomediatypecrt)
target_link_options(OmniphonyStreamAPO PRIVATE /MANIFEST:NO)
set_target_properties(OmniphonyStreamAPO PROPERTIES PREFIX "" OUTPUT_NAME "OmniphonyStreamAPO")

add_executable(OmniphonyApoCtl OmniphonyApoCtlAcl.cpp)
target_compile_features(OmniphonyApoCtl PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyApoCtl PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyApoCtl PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyApoCtl PRIVATE ole32 uuid propsys advapi32)

add_executable(OmniphonyApoSmoke OmniphonyApoSmoke.cpp)
target_compile_features(OmniphonyApoSmoke PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyApoSmoke PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyApoSmoke PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyApoSmoke PRIVATE ole32 uuid audioeng audiomediatypecrt)

add_executable(OmniphonyStreamApoSmoke OmniphonyStreamApoSmoke.cpp)
target_compile_features(OmniphonyStreamApoSmoke PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyStreamApoSmoke PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyStreamApoSmoke PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyStreamApoSmoke PRIVATE ole32 uuid audioeng audiomediatypecrt)

add_executable(OmniphonyRealtimeSmoke OmniphonyRealtimeSmoke.cpp)
target_compile_features(OmniphonyRealtimeSmoke PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyRealtimeSmoke PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyRealtimeSmoke PRIVATE /W4 /WX /permissive-)
target_include_directories(OmniphonyRealtimeSmoke PRIVATE "${OMNIPHONY_REALTIME_INCLUDE}")

add_executable(OmniphonyMixProbe OmniphonyMixProbe.cpp)
target_compile_features(OmniphonyMixProbe PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyMixProbe PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyMixProbe PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyMixProbe PRIVATE ole32 uuid propsys advapi32)

# Production acceptance deliberately uses a separate, read-only WASAPI probe.
# Unlike the development MixProbe, it owns no registry/ACL repair path.
add_executable(OmniphonyProductionProbe OmniphonyProductionProbe.cpp)
target_compile_features(OmniphonyProductionProbe PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyProductionProbe PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyProductionProbe PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyProductionProbe PRIVATE ole32 uuid)
