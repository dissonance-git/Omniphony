cmake_minimum_required(VERSION 3.20)
project(OmniphonyEndpointApo LANGUAGES CXX)

if(NOT WIN32)
    message(FATAL_ERROR "The Omniphony endpoint APO is Windows-only.")
endif()

set(OMNIPHONY_REALTIME_INCLUDE "${CMAKE_CURRENT_SOURCE_DIR}/../../realtime_ffi/include")

add_library(OmniphonyAPO SHARED
    OmniphonyAPO.cpp
    OmniphonyAPO.def
)
target_compile_features(OmniphonyAPO PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyAPO PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyAPO PRIVATE /W4 /WX /permissive-)
target_include_directories(OmniphonyAPO PRIVATE "${OMNIPHONY_REALTIME_INCLUDE}")
target_link_libraries(OmniphonyAPO PRIVATE ole32 uuid advapi32 audioeng AudioBaseProcessingObjectV140 legacy_stdio_definitions audiomediatypecrt)
# Protected AudioDG can reject custom APO DLLs that carry an embedded manifest.
# Keep the binary manifest-free and validate that contract in CI.
target_link_options(OmniphonyAPO PRIVATE /MANIFEST:NO)
set_target_properties(OmniphonyAPO PROPERTIES PREFIX "" OUTPUT_NAME "OmniphonyAPO")

add_executable(OmniphonyApoCtl OmniphonyApoCtl.cpp)
target_compile_features(OmniphonyApoCtl PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyApoCtl PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyApoCtl PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyApoCtl PRIVATE ole32 uuid propsys advapi32)

add_executable(OmniphonyApoSmoke OmniphonyApoSmoke.cpp)
target_compile_features(OmniphonyApoSmoke PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyApoSmoke PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyApoSmoke PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyApoSmoke PRIVATE ole32 uuid audioeng audiomediatypecrt)

add_executable(OmniphonyRealtimeSmoke OmniphonyRealtimeSmoke.cpp)
target_compile_features(OmniphonyRealtimeSmoke PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyRealtimeSmoke PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyRealtimeSmoke PRIVATE /W4 /WX /permissive-)
target_include_directories(OmniphonyRealtimeSmoke PRIVATE "${OMNIPHONY_REALTIME_INCLUDE}")

add_executable(OmniphonyMixProbe OmniphonyMixProbe.cpp)
target_compile_features(OmniphonyMixProbe PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyMixProbe PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyMixProbe PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyMixProbe PRIVATE ole32 uuid propsys)
