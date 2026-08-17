cmake_minimum_required(VERSION 3.20)
project(OmniphonyEapoBootstrap LANGUAGES CXX)

if(NOT WIN32)
    message(FATAL_ERROR "Omniphony Equalizer APO bootstrap is Windows-only")
endif()

add_library(OmniphonyVstBridge SHARED
    omniphony_vst_bridge.cpp
)

target_compile_features(OmniphonyVstBridge PRIVATE cxx_std_20)
target_compile_definitions(OmniphonyVstBridge PRIVATE
    WIN32_LEAN_AND_MEAN
    NOMINMAX
)
if(MSVC)
    target_compile_options(OmniphonyVstBridge PRIVATE /W4 /WX /permissive-)
endif()

set_target_properties(OmniphonyVstBridge PROPERTIES
    OUTPUT_NAME "OmniphonyVstBridge"
)

add_executable(omniphony_vst_bridge_smoke
    smoke_host.cpp
)
target_compile_features(omniphony_vst_bridge_smoke PRIVATE cxx_std_20)
target_compile_definitions(omniphony_vst_bridge_smoke PRIVATE
    WIN32_LEAN_AND_MEAN
    NOMINMAX
)
if(MSVC)
    target_compile_options(omniphony_vst_bridge_smoke PRIVATE /W4 /WX /permissive-)
endif()
