cmake_minimum_required(VERSION 3.16)

project(obliteration)

# Import VCPKG.
set(CMAKE_TOOLCHAIN_FILE ${CMAKE_CURRENT_SOURCE_DIR}/vcpkg/scripts/buildsystems/vcpkg.cmake CACHE STRING "Vcpkg toolchain file")

# Set warning level to highest. Thie will propagate to sub-directories too.
if(WIN32)
    add_compile_options(/W4)
else()
    add_compile_options(-Wall -Wextra)
endif()

add_subdirectory(src)
