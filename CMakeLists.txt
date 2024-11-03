cmake_minimum_required(VERSION 3.21)

include(FetchContent)
include(cmake/cargo.cmake)

# Fetch external CMake scripts.
FetchContent_Declare(
    msvc
    GIT_REPOSITORY https://github.com/MarkSchofield/WindowsToolchain.git
    GIT_TAG v0.11.0
    GIT_SHALLOW ON
)

FetchContent_MakeAvailable(msvc)

# Set project properties.
project(obliteration)

# Set warning level to highest. This will propagate to sub-directories too.
if(WIN32)
    add_compile_options(/W4)
else()
    add_compile_options(-Wall -Wextra)
endif()

# Fix warning for DOWNLOAD_EXTRACT_TIMESTAMP on ExternalProject.
if (CMAKE_VERSION VERSION_GREATER_EQUAL "3.24.0")
    cmake_policy(SET CMP0135 NEW)
endif()

# Setup Rust targets.
if(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "arm64" OR ${CMAKE_SYSTEM_PROCESSOR} STREQUAL "aarch64")
    # Pre-compiled core crate for aarch64-unknown-none-softfloat does not support
    # Position-Independent Executable so we need nightly toolchain for build-std feature to
    # re-build core crate to support Position-Independent Executable.
    set(kernel_env "softfloat")
    set(kernel_toolchain "nightly")
    set(kernel_opts -Z build-std=core,alloc)
else()
    set(kernel_env "")
endif()

add_cargo(MANIFEST Cargo.toml)
add_crate(gui LIBRARY)
add_crate(obkrnl
    TOOLCHAIN ${kernel_toolchain}
    VENDOR "unknown"
    OPERATING_SYSTEM "none"
    ENVIRONMENT "${kernel_env}"
    ARGS ${kernel_opts})

add_dependencies(gui obkrnl)

# Add GUI.
add_subdirectory(gui)
