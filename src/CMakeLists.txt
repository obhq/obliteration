cmake_minimum_required(VERSION 3.12)

project(obliteration)

set(CMAKE_TOOLCHAIN_FILE "${CMAKE_CURRENT_SOURCE_DIR}/../vcpkg/scripts/buildsystems/vcpkg.cmake" CACHE STRING "Vcpkg toolchain file")

# Set warning level to maximum (including sub-directories).
if(WIN32)
    add_compile_options(/Wall)
else()
    add_compile_options(-Wall -Wextra)
endif()

# External dependencies.
find_package(Qt5 COMPONENTS Widgets REQUIRED)

# Setup application target.
add_executable(obliteration main.cpp main_window.cpp)

set_property(TARGET obliteration PROPERTY AUTOMOC ON)
set_property(TARGET obliteration PROPERTY AUTORCC ON)
set_property(TARGET obliteration PROPERTY AUTOUIC ON)

target_compile_features(obliteration PRIVATE cxx_std_17)

target_link_libraries(obliteration PRIVATE Qt5::Widgets)

# Setup installation.
install(TARGETS obliteration DESTINATION .)
