# CMake modules
include(ExternalProject)

# External dependencies.
find_package(Qt6 COMPONENTS Widgets REQUIRED)
find_package(Threads REQUIRED)

# Setup Rust target.
if(WIN32)
    set(LIBCORE $<IF:$<CONFIG:Debug>,${CMAKE_CURRENT_SOURCE_DIR}/target/debug/core.lib,${CMAKE_CURRENT_SOURCE_DIR}/target/release/core.lib>)
else()
    set(LIBCORE $<IF:$<CONFIG:Debug>,${CMAKE_CURRENT_SOURCE_DIR}/target/debug/libcore.a,${CMAKE_CURRENT_SOURCE_DIR}/target/release/libcore.a>)
endif()

ExternalProject_Add(libcore
    SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}
    CONFIGURE_COMMAND ""
    BUILD_COMMAND cargo build $<IF:$<CONFIG:Debug>,-v,--release>
    BUILD_IN_SOURCE ON
    BUILD_ALWAYS ON
    BUILD_BYPRODUCTS ${LIBCORE}
    INSTALL_COMMAND ""
    TEST_COMMAND cargo test
    TEST_EXCLUDE_FROM_MAIN ON)

# Setup application target.
add_executable(obliteration WIN32
    game_models.cpp
    game_settings.cpp
    game_settings_dialog.cpp
    initialize_dialog.cpp
    main.cpp
    main_window.cpp
    resources.qrc
    settings.cpp
    util.cpp)

if(WIN32)
    target_sources(obliteration PRIVATE resources.rc)
endif()

add_dependencies(obliteration libcore)

set_property(TARGET obliteration PROPERTY AUTOMOC ON)
set_property(TARGET obliteration PROPERTY AUTORCC ON)

target_compile_features(obliteration PRIVATE cxx_std_17)

target_link_libraries(obliteration PRIVATE Qt6::Widgets)
target_link_libraries(obliteration PRIVATE Threads::Threads)
target_link_libraries(obliteration PRIVATE ${LIBCORE})

if(WIN32)
    target_link_libraries(obliteration PRIVATE bcrypt imm32 setupapi userenv version winmm ws2_32)
else()
    target_link_libraries(obliteration PRIVATE ${CMAKE_DL_LIBS})
endif()

# Setup installation.
if(WIN32)
    install(TARGETS obliteration DESTINATION .)
else()
    install(TARGETS obliteration DESTINATION bin)
endif()

install(SCRIPT post-install.cmake)
