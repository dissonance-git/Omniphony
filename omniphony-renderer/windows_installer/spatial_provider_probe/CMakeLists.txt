cmake_minimum_required(VERSION 3.20)
project(OmniphonySpatialProviderProbe LANGUAGES CXX)

if(NOT WIN32)
    message(FATAL_ERROR "The Omniphony Spatial Provider Probe is Windows-only.")
endif()

add_library(OmniphonySpatialProbe SHARED
    OmniphonySpatialProbe.cpp
    OmniphonySpatialProbe.def
)
target_compile_features(OmniphonySpatialProbe PRIVATE cxx_std_17)
target_compile_definitions(OmniphonySpatialProbe PRIVATE UNICODE _UNICODE)
target_compile_options(OmniphonySpatialProbe PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonySpatialProbe PRIVATE ole32 uuid)
set_target_properties(OmniphonySpatialProbe PROPERTIES
    PREFIX ""
    OUTPUT_NAME "OmniphonySpatialProbe"
)

add_executable(OmniphonySpatialProbeCtl
    OmniphonySpatialProbeCtl.cpp
)
target_compile_features(OmniphonySpatialProbeCtl PRIVATE cxx_std_17)
target_compile_definitions(OmniphonySpatialProbeCtl PRIVATE UNICODE _UNICODE)
target_compile_options(OmniphonySpatialProbeCtl PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonySpatialProbeCtl PRIVATE ole32 uuid advapi32)
