cmake_minimum_required(VERSION 3.20)
cmake_policy(SET CMP0091 NEW)
project(OmniphonySpatialProviderCanary LANGUAGES CXX)

if(NOT WIN32)
    message(FATAL_ERROR "The spatial provider canary is Windows-only.")
endif()

set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")

add_library(OmniphonySpatialProviderCanary SHARED
    OmniphonySpatialProviderCanary.cpp
    OmniphonySpatialProviderCanary.def
)
target_compile_features(OmniphonySpatialProviderCanary PRIVATE cxx_std_17)
target_compile_definitions(OmniphonySpatialProviderCanary PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonySpatialProviderCanary PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonySpatialProviderCanary PRIVATE ole32)
target_link_options(OmniphonySpatialProviderCanary PRIVATE /MANIFEST:NO)
set_target_properties(OmniphonySpatialProviderCanary PROPERTIES PREFIX "" OUTPUT_NAME "OmniphonySpatialProviderCanary")

add_executable(OmniphonySpatialProviderCanaryCtl
    OmniphonySpatialProviderCanaryCtl.cpp
)
target_compile_features(OmniphonySpatialProviderCanaryCtl PRIVATE cxx_std_17)
target_compile_definitions(OmniphonySpatialProviderCanaryCtl PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonySpatialProviderCanaryCtl PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonySpatialProviderCanaryCtl PRIVATE ole32 advapi32)
