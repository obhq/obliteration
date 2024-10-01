cmake_minimum_required(VERSION 3.21)

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
if(WIN32)
    if(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "AMD64")
        set(KERNEL_TARGET x86_64-unknown-none)
    else()
        message(FATAL_ERROR "Target CPU is not supported")
    endif()
else()
    if(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "x86_64")
        set(KERNEL_TARGET x86_64-unknown-none)
    elseif(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "arm64" OR ${CMAKE_SYSTEM_PROCESSOR} STREQUAL "aarch64")
        # Pre-compiled core crate for aarch64-unknown-none-softfloat does not support
        # Position-Independent Executable so we need nightly toolchain for build-std feature to
        # re-build core crate to support Position-Independent Executable.
        set(KERNEL_TARGET aarch64-unknown-none-softfloat)
        set(KERNEL_TOOLCHAIN +nightly)
        set(KERNEL_OPTS -Z build-std=core,alloc)
    else()
        message(FATAL_ERROR "Target CPU is not supported")
    endif()
endif()

set(KERNEL_OUTPUTS $<IF:$<CONFIG:Debug>,${CMAKE_CURRENT_SOURCE_DIR}/target/${KERNEL_TARGET}/debug,${CMAKE_CURRENT_SOURCE_DIR}/target/${KERNEL_TARGET}/release>)
set(KERNEL ${KERNEL_OUTPUTS}/obkrnl)
set(HOST_OUTPUTS $<IF:$<CONFIG:Debug>,${CMAKE_CURRENT_SOURCE_DIR}/target/debug,${CMAKE_CURRENT_SOURCE_DIR}/target/release>)

if(WIN32)
    set(LIBGUI ${HOST_OUTPUTS}/gui.lib)
else()
    set(LIBGUI ${HOST_OUTPUTS}/libgui.a)
endif()

add_custom_target(libgui
    COMMAND cargo build $<IF:$<CONFIG:Debug>,--profile=dev,--release>
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/gui
    BYPRODUCTS ${LIBGUI})

add_custom_target(kernel
    COMMAND cargo ${KERNEL_TOOLCHAIN} build $<IF:$<CONFIG:Debug>,--profile=dev,--release> --target ${KERNEL_TARGET} ${KERNEL_OPTS}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/src/obkrnl
    BYPRODUCTS ${KERNEL})

add_dependencies(libgui kernel)

# Add GUI.
add_subdirectory(src)
