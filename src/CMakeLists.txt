# CMake modules
include(ExternalProject)

# External dependencies.
find_package(Qt5 COMPONENTS Widgets REQUIRED)

# Setup Rust target.
ExternalProject_Add(emulator
    SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}
    CONFIGURE_COMMAND ""
    BUILD_COMMAND cargo build $<IF:$<CONFIG:Debug>,-v,--release>
    BUILD_IN_SOURCE ON
    BUILD_ALWAYS ON
    INSTALL_COMMAND ""
    TEST_COMMAND cargo test
    TEST_EXCLUDE_FROM_MAIN ON)

if(WIN32)
    set(EMULATOR_DEBUG ${CMAKE_CURRENT_SOURCE_DIR}/target/debug/obliteration.lib)
    set(EMULATOR_RELEASE ${CMAKE_CURRENT_SOURCE_DIR}/target/release/obliteration.lib)
else()
    set(EMULATOR_DEBUG ${CMAKE_CURRENT_SOURCE_DIR}/target/debug/libobliteration.a)
    set(EMULATOR_RELEASE ${CMAKE_CURRENT_SOURCE_DIR}/target/release/libobliteration.a)
endif()

# Setup application target.
add_executable(obliteration WIN32
    game_models.cpp
    initialize_dialog.cpp
    main.cpp
    main_window.cpp
    settings.cpp)

add_dependencies(obliteration emulator)

set_property(TARGET obliteration PROPERTY AUTOMOC ON)
set_property(TARGET obliteration PROPERTY AUTORCC ON)

target_compile_features(obliteration PRIVATE cxx_std_17)

target_link_libraries(obliteration PRIVATE Qt5::Widgets)
target_link_libraries(obliteration PRIVATE debug ${EMULATOR_DEBUG} optimized ${EMULATOR_RELEASE})

# Setup installation.
if(WIN32)
    install(TARGETS obliteration DESTINATION .)
else()
    install(TARGETS obliteration DESTINATION bin)
endif()

install(SCRIPT post-install.cmake)
