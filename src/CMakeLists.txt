cmake_minimum_required(VERSION 3.16)

project(obliteration)

set(CMAKE_TOOLCHAIN_FILE "${CMAKE_CURRENT_SOURCE_DIR}/../vcpkg/scripts/buildsystems/vcpkg.cmake" CACHE STRING "Vcpkg toolchain file")

# Set warning level to maximum (including sub-directories).
if(WIN32)
    add_compile_options(/W4)
else()
    add_compile_options(-Wall -Wextra)
endif()

# External dependencies.
find_package(Qt5 COMPONENTS Widgets REQUIRED)

# Setup application target.
add_executable(obliteration WIN32
    initialize_dialog.cpp
    initialize_dialog.ui
    main.cpp
    main_window.cpp
    settings.cpp)

set_property(TARGET obliteration PROPERTY AUTOMOC ON)
set_property(TARGET obliteration PROPERTY AUTORCC ON)
set_property(TARGET obliteration PROPERTY AUTOUIC ON)

target_compile_features(obliteration PRIVATE cxx_std_17)

target_link_libraries(obliteration PRIVATE Qt5::Widgets)

# Setup installation.
if(WIN32)
    install(TARGETS obliteration DESTINATION .)
else()
    install(TARGETS obliteration DESTINATION bin)
endif()

install(SCRIPT post-install.cmake)
