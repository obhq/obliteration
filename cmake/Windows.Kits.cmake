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
# | CMake Variable                                      | Description                                                                                                                                       |
# |-----------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
# | CMAKE_SYSTEM_VERSION                                | The version of the operating system for which CMake is to build. Defaults to the host version.                                                    |
# | CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE         | The architecture of the tooling to use. Defaults to x64.                                                                                          |
# | CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION            | The version of the Windows SDK to use. Defaults to the highest installed, that is no higher than CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION_MAXIMUM |
# | CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION_MAXIMUM    | The maximum version of the Windows SDK to use, for example '10.0.19041.0'. Defaults to nothing                                                    |
# | CMAKE_WINDOWS_KITS_10_DIR                           | The location of the root of the Windows Kits 10 directory.                                                                                        |
#
# The following variables will be set:
#
# | CMake Variable                              | Description                                                                                           |
# |---------------------------------------------|-------------------------------------------------------------------------------------------------------|
# | CMAKE_MT                                    | The path to the 'mt' tool.                                                                            |
# | CMAKE_RC_COMPILER                           | The path to the 'rc' tool.                                                                            |
# | CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION    | The version of the Windows SDK to be used.                                                            |
# | MDMERGE_TOOL                                | The path to the 'mdmerge' tool.                                                                       |
# | MIDL_COMPILER                               | The path to the 'midl' compiler.                                                                      |
# | WINDOWS_KITS_BIN_PATH                       | The path to the folder containing the Windows Kits binaries.                                          |
# | WINDOWS_KITS_INCLUDE_PATH                   | The path to the folder containing the Windows Kits include files.                                     |
# | WINDOWS_KITS_LIB_PATH                       | The path to the folder containing the Windows Kits library files.                                     |
# | WINDOWS_KITS_REFERENCES_PATH                | The path to the folder containing the Windows Kits references.                                        |
#
include_guard()

if(NOT CMAKE_SYSTEM_VERSION)
    set(CMAKE_SYSTEM_VERSION ${CMAKE_HOST_SYSTEM_VERSION})
endif()

if(NOT CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE)
    set(CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE x64)
endif()

if(NOT CMAKE_WINDOWS_KITS_10_DIR)
    get_filename_component(CMAKE_WINDOWS_KITS_10_DIR "[HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Microsoft SDKs\\Windows\\v10.0;InstallationFolder]" ABSOLUTE CACHE)
    if ("${CMAKE_WINDOWS_KITS_10_DIR}" STREQUAL "/registry")
        unset(CMAKE_WINDOWS_KITS_10_DIR)
    endif()
endif()

if(NOT CMAKE_WINDOWS_KITS_10_DIR)
    message(FATAL_ERROR "Unable to find an installed Windows SDK, and one wasn't specified.")
endif()

# If a CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION wasn't specified, find the highest installed version that is no higher
# than the host version
if(NOT CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION)
    file(GLOB WINDOWS_KITS_VERSIONS RELATIVE "${CMAKE_WINDOWS_KITS_10_DIR}/lib" "${CMAKE_WINDOWS_KITS_10_DIR}/lib/*")
    list(FILTER WINDOWS_KITS_VERSIONS INCLUDE REGEX "10\\.0\\.")
    list(SORT WINDOWS_KITS_VERSIONS COMPARE NATURAL ORDER DESCENDING)
    while(WINDOWS_KITS_VERSIONS)
        list(POP_FRONT WINDOWS_KITS_VERSIONS CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION)
        if(NOT CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION_MAXIMUM)
            message(VERBOSE "Windows.Kits: Defaulting version: ${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}")
            break()
        endif()

        if(CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION VERSION_LESS_EQUAL CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION_MAXIMUM)
            message(VERBOSE "Windows.Kits: Choosing version: ${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}")
            break()
        endif()

        message(VERBOSE "Windows.Kits: Not suitable: ${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}")
        set(CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION)
    endwhile()
endif()

if(NOT CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION)
    message(FATAL_ERROR "A Windows SDK could not be found.")
endif()

set(WINDOWS_KITS_BIN_PATH "${CMAKE_WINDOWS_KITS_10_DIR}/bin/${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}" CACHE PATH "" FORCE)
set(WINDOWS_KITS_INCLUDE_PATH "${CMAKE_WINDOWS_KITS_10_DIR}/include/${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}" CACHE PATH "" FORCE)
set(WINDOWS_KITS_LIB_PATH "${CMAKE_WINDOWS_KITS_10_DIR}/lib/${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION}" CACHE PATH "" FORCE)
set(WINDOWS_KITS_REFERENCES_PATH "${CMAKE_WINDOWS_KITS_10_DIR}/References" CACHE PATH "" FORCE)
set(WINDOWS_KITS_PLATFORM_PATH "${CMAKE_WINDOWS_KITS_10_DIR}/Platforms/UAP/${CMAKE_SYSTEM_VERSION}/Platform.xml" CACHE PATH "" FORCE)

if(NOT EXISTS ${WINDOWS_KITS_BIN_PATH})
    message(FATAL_ERROR "Windows SDK ${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION} cannot be found: Folder '${WINDOWS_KITS_BIN_PATH}' does not exist.")
endif()

if(NOT EXISTS ${WINDOWS_KITS_INCLUDE_PATH})
    message(FATAL_ERROR "Windows SDK ${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION} cannot be found: Folder '${WINDOWS_KITS_INCLUDE_PATH}' does not exist.")
endif()

if(NOT EXISTS ${WINDOWS_KITS_LIB_PATH})
    message(FATAL_ERROR "Windows SDK ${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION} cannot be found: Folder '${WINDOWS_KITS_LIB_PATH}' does not exist.")
endif()

set(CMAKE_MT "${WINDOWS_KITS_BIN_PATH}/${CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE}/mt.exe")
set(CMAKE_RC_COMPILER_INIT "${WINDOWS_KITS_BIN_PATH}/${CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE}/rc.exe")
set(CMAKE_RC_FLAGS_INIT "/nologo")

set(MIDL_COMPILER "${WINDOWS_KITS_BIN_PATH}/${CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE}/midl.exe")
set(MDMERGE_TOOL "${WINDOWS_KITS_BIN_PATH}/${CMAKE_VS_PLATFORM_TOOLSET_HOST_ARCHITECTURE}/mdmerge.exe")

# Windows SDK
if((CMAKE_SYSTEM_PROCESSOR STREQUAL AMD64) OR (CMAKE_SYSTEM_PROCESSOR STREQUAL x64))
    set(WINDOWS_KITS_TARGET_ARCHITECTURE x64)
elseif((CMAKE_SYSTEM_PROCESSOR STREQUAL arm)
    OR (CMAKE_SYSTEM_PROCESSOR STREQUAL arm64)
    OR (CMAKE_SYSTEM_PROCESSOR STREQUAL x86))
    set(WINDOWS_KITS_TARGET_ARCHITECTURE ${CMAKE_SYSTEM_PROCESSOR})
else()
    message(FATAL_ERROR "Unable identify Windows Kits architecture for CMAKE_SYSTEM_PROCESSOR ${CMAKE_SYSTEM_PROCESSOR}")
endif()

foreach(LANG C CXX RC)
    list(APPEND CMAKE_${LANG}_STANDARD_INCLUDE_DIRECTORIES "${WINDOWS_KITS_INCLUDE_PATH}/ucrt")
    list(APPEND CMAKE_${LANG}_STANDARD_INCLUDE_DIRECTORIES "${WINDOWS_KITS_INCLUDE_PATH}/shared")
    list(APPEND CMAKE_${LANG}_STANDARD_INCLUDE_DIRECTORIES "${WINDOWS_KITS_INCLUDE_PATH}/um")
    list(APPEND CMAKE_${LANG}_STANDARD_INCLUDE_DIRECTORIES "${WINDOWS_KITS_INCLUDE_PATH}/winrt")
    list(APPEND CMAKE_${LANG}_STANDARD_INCLUDE_DIRECTORIES "${WINDOWS_KITS_INCLUDE_PATH}/cppwinrt")
endforeach()

link_directories("${WINDOWS_KITS_LIB_PATH}/ucrt/${WINDOWS_KITS_TARGET_ARCHITECTURE}")
link_directories("${WINDOWS_KITS_LIB_PATH}/um/${WINDOWS_KITS_TARGET_ARCHITECTURE}")
link_directories("${WINDOWS_KITS_REFERENCES_PATH}/${WINDOWS_KITS_TARGET_ARCHITECTURE}")
