# CMake modules
include(ExternalProject)

# External dependencies.
find_package(Qt6 COMPONENTS Widgets REQUIRED)
find_package(Threads REQUIRED)

# Setup Rust target.
set(RUST_OUTPUTS $<IF:$<CONFIG:Debug>,${CMAKE_CURRENT_SOURCE_DIR}/target/debug,${CMAKE_CURRENT_SOURCE_DIR}/target/release>)
set(KERNEL ${RUST_OUTPUTS}/obkrnl${CMAKE_EXECUTABLE_SUFFIX})

if(WIN32)
    set(LIBCORE ${RUST_OUTPUTS}/core.lib)
else()
    set(LIBCORE ${RUST_OUTPUTS}/libcore.a)
endif()

add_custom_target(core
    COMMAND cargo build $<IF:$<CONFIG:Debug>,--profile=dev,--release>
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

# Setup application target.
add_executable(obliteration WIN32 MACOSX_BUNDLE
    ansi_escape.cpp
    app_data.cpp
    core.cpp
    game_graphic_settings.cpp
    game_models.cpp
    game_settings.cpp
    game_settings_dialog.cpp
    initialize_wizard.cpp
    log_formatter.cpp
    main.cpp
    main_window.cpp
    path.cpp
    progress_dialog.cpp
    resources.qrc
    settings.cpp
    system.cpp
    system_downloader.cpp
    update_firmware.cpp)

if(WIN32)
    target_sources(obliteration PRIVATE resources.rc)
elseif(APPLE)
    target_sources(obliteration PRIVATE resources/obliteration.icns)
endif()

add_dependencies(obliteration core)

set_target_properties(obliteration PROPERTIES AUTOMOC ON AUTORCC ON)

if(WIN32)
    set_target_properties(obliteration PROPERTIES OUTPUT_NAME Obliteration)
endif()

if(APPLE)
    set_target_properties(obliteration PROPERTIES
        MACOSX_BUNDLE_GUI_IDENTIFIER io.github.obhq.obliteration
        MACOSX_BUNDLE_BUNDLE_NAME Obliteration
        MACOSX_BUNDLE_BUNDLE_VERSION 0.1.0
        MACOSX_BUNDLE_SHORT_VERSION_STRING 0.1.0
        MACOSX_BUNDLE_COPYRIGHT "Copyright © Obliteration Contributors"
        MACOSX_BUNDLE_ICON_FILE obliteration
        RESOURCE resources/obliteration.icns)
endif()

target_compile_features(obliteration PRIVATE cxx_std_17)

target_link_libraries(obliteration PRIVATE Qt6::Widgets)
target_link_libraries(obliteration PRIVATE Threads::Threads)
target_link_libraries(obliteration PRIVATE ${LIBCORE})

if(WIN32)
    target_link_libraries(obliteration PRIVATE bcrypt imm32 ntdll setupapi userenv version winmm ws2_32)
else()
    target_link_libraries(obliteration PRIVATE ${CMAKE_DL_LIBS})
endif()

# Setup installation.
if(WIN32)
    install(TARGETS obliteration DESTINATION .)
else()
    install(TARGETS obliteration BUNDLE DESTINATION . RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
endif()

if(WIN32)
    install(SCRIPT deploy-win32.cmake)
elseif(APPLE)
    qt_generate_deploy_app_script(
        TARGET obliteration
        OUTPUT_SCRIPT QT_DEPLOY_SCRIPT
        NO_TRANSLATIONS
        NO_COMPILER_RUNTIME)

    install(SCRIPT ${QT_DEPLOY_SCRIPT})
endif()

if(APPLE)
    install(PROGRAMS ${KERNEL} DESTINATION obliteration.app/Contents/MacOS)
else()
    install(PROGRAMS ${KERNEL} TYPE BIN)
endif()
