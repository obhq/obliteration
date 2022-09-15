cmake_minimum_required(VERSION 3.12)

project(obliteration)

set(CMAKE_TOOLCHAIN_FILE "${CMAKE_CURRENT_SOURCE_DIR}/../vcpkg/scripts/buildsystems/vcpkg.cmake" CACHE STRING "Vcpkg toolchain file")

add_executable(obliteration main.cpp) 

target_compile_features(obliteration PUBLIC cxx_std_17)

install(TARGETS obliteration DESTINATION .)
