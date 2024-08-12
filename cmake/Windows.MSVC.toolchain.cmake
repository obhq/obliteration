#----------------------------------------------------------------------------------------------------------------------
# MIT License
#
# Copyright (c) 2021 Mark Schofield
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.
#----------------------------------------------------------------------------------------------------------------------
#
# This CMake toolchain file configures a CMake, non-'Visual Studio Generator' build to use
# the MSVC compilers and tools.
#
# The following variables can be used to configure the behavior of this toolchain file:
#
# | CMake Variable                              | Description                                                                                                              |
# |---------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
# | CMAKE_SYSTEM_PROCESSOR                      | The processor to compiler for. One of 'x86', 'x64'/'AMD64', 'arm', 'arm64'. Defaults to ${CMAKE_HOST_SYSTEM_PROCESSOR}.  |
# | CMAKE_SYSTEM_VERSION                        | The version of the operating system for which CMake is to build. Defaults to the host version.                           |
# | CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE | The architecture of the toolset to use. Defaults to 'x64'.                                                               |
# | CMAKE_VS_PRODUCTS                           | One or more Visual Studio Product IDs to consider. Defaults to '*'                                                       |
# | CMAKE_VS_VERSION_PRERELEASE                 | Whether 'prerelease' versions of Visual Studio should be considered. Defaults to 'OFF'                                   |
# | CMAKE_VS_VERSION_RANGE                      | A verson range for VS instances to find. For example, '[16.0,17.0)' will find versions '16.*'. Defaults to '[16.0,17.0)' |
# | CMAKE_WINDOWS_KITS_10_DIR                   | The location of the root of the Windows Kits 10 directory.                                                               |
# | VS_EXPERIMENTAL_MODULE                      | Whether experimental module support should be enabled.                                                                   |
# | VS_INSTALLATION_PATH                        | The location of the root of the Visual Studio installation. If not specified VSWhere will be used to search for one.     |
# | VS_PLATFORM_TOOLSET_VERSION                 | The version of the MSVC toolset to use. For example, 14.29.30133. Defaults to the highest available.                     |
# | VS_USE_SPECTRE_MITIGATION_ATLMFC_RUNTIME    | Whether the compiler should link with the ATLMFC runtime that uses 'Spectre' mitigations. Defaults to 'OFF'.             |
# | VS_USE_SPECTRE_MITIGATION_RUNTIME           | Whether the compiler should link with a runtime that uses 'Spectre' mitigations. Defaults to 'OFF'.                      |
#
# The toolchain file will set the following variables:
#
# | CMake Variable                              | Description                                                                                           |
# |---------------------------------------------|-------------------------------------------------------------------------------------------------------|
# | CMAKE_C_COMPILER                            | The path to the C compiler to use.                                                                    |
# | CMAKE_CXX_COMPILER                          | The path to the C++ compiler to use.                                                                  |
# | CMAKE_MT                                    | The path to the 'mt.exe' tool to use.                                                                 |
# | CMAKE_RC_COMPILER                           | The path tp the 'rc.exe' tool to use.                                                                 |
# | CMAKE_SYSTEM_NAME                           | Windows                                                                                               |
# | CMAKE_VS_PLATFORM_TOOLSET_VERSION           | The version of the MSVC toolset being used - e.g. 14.29.30133.                                        |
# | WIN32                                       | 1                                                                                                     |
# | MSVC                                        | 1                                                                                                     |
# | MSVC_VERSION                                | The '<major><minor>' version of the C++ compiler being used. For example, '1929'                      |
#
# Resources:
#   <https://cmake.org/cmake/help/latest/manual/cmake-toolchains.7.html>
#
cmake_minimum_required(VERSION 3.20)

include_guard()

if(NOT (CMAKE_HOST_SYSTEM_NAME STREQUAL Windows))
    return()
endif()

set(UNUSED ${CMAKE_TOOLCHAIN_FILE}) # Note: only to prevent cmake unused variable warninig
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_TRY_COMPILE_PLATFORM_VARIABLES
    CMAKE_CROSSCOMPILING
    CMAKE_SYSTEM_PROCESSOR
    CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE
    CMAKE_VS_PRODUCTS
    CMAKE_VS_VERSION_PRERELEASE
    CMAKE_VS_VERSION_RANGE
    VS_INSTALLATION_PATH
    VS_INSTALLATION_VERSION
    VS_PLATFORM_TOOLSET_VERSION
)
set(CMAKE_CROSSCOMPILING TRUE)
set(WIN32 1)
set(MSVC 1)

include("${CMAKE_CURRENT_LIST_DIR}/VSWhere.cmake")

if(NOT CMAKE_SYSTEM_PROCESSOR)
    set(CMAKE_SYSTEM_PROCESSOR ${CMAKE_HOST_SYSTEM_PROCESSOR})
endif()

if(NOT CMAKE_VS_VERSION_RANGE)
    set(CMAKE_VS_VERSION_RANGE "[16.0,)")
endif()

if(NOT CMAKE_VS_VERSION_PRERELEASE)
    set(CMAKE_VS_VERSION_PRERELEASE OFF)
endif()

if(NOT CMAKE_VS_PRODUCTS)
    set(CMAKE_VS_PRODUCTS "*")
endif()

if(NOT CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE)
    set(CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE x64)
endif()

if(NOT VS_USE_SPECTRE_MITIGATION_RUNTIME)
    set(VS_USE_SPECTRE_MITIGATION_RUNTIME OFF)
endif()

# Find Visual Studio
#
if(NOT VS_INSTALLATION_PATH)
    findVisualStudio(
        VERSION ${CMAKE_VS_VERSION_RANGE}
        PRERELEASE ${CMAKE_VS_VERSION_PRERELEASE}
        PRODUCTS ${CMAKE_VS_PRODUCTS}
        PROPERTIES
            installationVersion VS_INSTALLATION_VERSION
            installationPath VS_INSTALLATION_PATH
    )
endif()

message(VERBOSE "VS_INSTALLATION_VERSION = ${VS_INSTALLATION_VERSION}")
message(VERBOSE "VS_INSTALLATION_PATH = ${VS_INSTALLATION_PATH}")

if(NOT VS_INSTALLATION_PATH)
    message(FATAL_ERROR "Unable to find Visual Studio")
endif()

cmake_path(NORMAL_PATH VS_INSTALLATION_PATH)

set(VS_MSVC_PATH "${VS_INSTALLATION_PATH}/VC/Tools/MSVC")

# Use 'VS_PLATFORM_TOOLSET_VERSION' to resolve 'CMAKE_VS_PLATFORM_TOOLSET_VERSION'
#
if(NOT VS_PLATFORM_TOOLSET_VERSION)
    if(VS_TOOLSET_VERSION)
        message(WARNING "Old versions of WindowsToolchain incorrectly used 'VS_TOOLSET_VERSION' to specify the VS toolset version. This functionality is being deprecated - please use 'VS_PLATFORM_TOOLSET_VERSION' instead.")
        set(VS_PLATFORM_TOOLSET_VERSION ${VS_TOOLSET_VERSION})
    else()
        file(GLOB VS_PLATFORM_TOOLSET_VERSIONS RELATIVE ${VS_MSVC_PATH} ${VS_MSVC_PATH}/*)
        list(SORT VS_PLATFORM_TOOLSET_VERSIONS COMPARE NATURAL ORDER DESCENDING)
        list(POP_FRONT VS_PLATFORM_TOOLSET_VERSIONS VS_PLATFORM_TOOLSET_VERSION)
        unset(VS_PLATFORM_TOOLSET_VERSIONS)
    endif()
endif()

set(CMAKE_VS_PLATFORM_TOOLSET_VERSION ${VS_PLATFORM_TOOLSET_VERSION})
set(VS_TOOLSET_PATH "${VS_INSTALLATION_PATH}/VC/Tools/MSVC/${CMAKE_VS_PLATFORM_TOOLSET_VERSION}")

# Set the tooling variables, include_directories and link_directories
#

# Map CMAKE_SYSTEM_PROCESSOR values to CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE that identifies the tools that should
# be used to produce code for the CMAKE_SYSTEM_PROCESSOR.
if((CMAKE_SYSTEM_PROCESSOR STREQUAL AMD64) OR (CMAKE_SYSTEM_PROCESSOR STREQUAL x64))
    set(CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE x64)
elseif((CMAKE_SYSTEM_PROCESSOR STREQUAL arm)
    OR (CMAKE_SYSTEM_PROCESSOR STREQUAL arm64)
    OR (CMAKE_SYSTEM_PROCESSOR STREQUAL x86))
    set(CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE ${CMAKE_SYSTEM_PROCESSOR})
else()
    message(FATAL_ERROR "Unable identify compiler architecture for CMAKE_SYSTEM_PROCESSOR ${CMAKE_SYSTEM_PROCESSOR}")
endif()

set(CMAKE_CXX_COMPILER "${VS_TOOLSET_PATH}/bin/Host${CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE}/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}/cl.exe")
set(CMAKE_C_COMPILER "${VS_TOOLSET_PATH}/bin/Host${CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE}/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}/cl.exe")

if(CMAKE_SYSTEM_PROCESSOR STREQUAL arm)
    set(CMAKE_CXX_FLAGS_INIT "${CMAKE_CXX_FLAGS_INIT} /EHsc")
endif()

# Compiler
foreach(LANG C CXX RC)
    list(APPEND CMAKE_${LANG}_STANDARD_INCLUDE_DIRECTORIES "${VS_TOOLSET_PATH}/ATLMFC/include")
    list(APPEND CMAKE_${LANG}_STANDARD_INCLUDE_DIRECTORIES "${VS_TOOLSET_PATH}/include")
endforeach()

if(VS_USE_SPECTRE_MITIGATION_ATLMFC_RUNTIME)
    # Ensure that the necessary folder and files are present before adding the 'link_directories'
    toolchain_validate_vs_files(
        DESCRIPTION "ATLMFC Spectre libraries"
        FOLDER "${VS_TOOLSET_PATH}/ATLMFC/lib/spectre/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}"
        FILES
            atls.lib
    )
    link_directories("${VS_TOOLSET_PATH}/ATLMFC/lib/spectre/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}")
else()
    link_directories("${VS_TOOLSET_PATH}/ATLMFC/lib/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}")
endif()

if(VS_USE_SPECTRE_MITIGATION_RUNTIME)
    # Ensure that the necessary folder and files are present before adding the 'link_directories'
    toolchain_validate_vs_files(
        DESCRIPTION "Spectre libraries"
        FOLDER "${VS_TOOLSET_PATH}/lib/spectre/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}"
        FILES
            msvcrt.lib vcruntime.lib vcruntimed.lib
    )
    link_directories("${VS_TOOLSET_PATH}/lib/spectre/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}")
else()
    link_directories("${VS_TOOLSET_PATH}/lib/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}")
endif()

link_directories("${VS_TOOLSET_PATH}/lib/x86/store/references")

# Module support
if(VS_EXPERIMENTAL_MODULE)
    set(CMAKE_CXX_FLAGS_INIT "${CMAKE_CXX_FLAGS_INIT} /experimental:module")
    set(CMAKE_CXX_FLAGS_INIT "${CMAKE_CXX_FLAGS_INIT} /stdIfcDir \"${VS_TOOLSET_PATH}/ifc/${CMAKE_VS_PLATFORM_TOOLSET_ARCHITECTURE}\"")
endif()

# Windows Kits
include("${CMAKE_CURRENT_LIST_DIR}/Windows.Kits.cmake")
