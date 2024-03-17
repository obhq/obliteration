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

add_subdirectory(src)
