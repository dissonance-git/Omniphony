cmake_minimum_required(VERSION 3.20)
project(OmniphonyEndpointApo LANGUAGES CXX)

if(NOT WIN32)
    message(FATAL_ERROR "The Omniphony endpoint APO is Windows-only.")
endif()

add_library(OmniphonyAPO SHARED
    OmniphonyAPO.cpp
    OmniphonyAPO.def
)
target_compile_features(OmniphonyAPO PRIVATE cxx_std_17)
target_compile_definitions(OmniphonyAPO PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(OmniphonyAPO PRIVATE /W4 /WX /permissive-)
target_link_libraries(OmniphonyAPO PRIVATE ole32 uuid advapi32 audioeng AudioBaseProcessingObject audiomediatypecrt)
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
target_link_libraries(OmniphonyApoSmoke PRIVATE ole32 uuid audioeng)
