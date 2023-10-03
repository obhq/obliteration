# CMake modules
include(ExternalProject)

# External dependencies.
find_package(Qt6 COMPONENTS Widgets REQUIRED)
find_package(Threads REQUIRED)

# Setup LLVM target.
set(LLVM_OPTS -DCMAKE_INSTALL_PREFIX:STRING=<INSTALL_DIR> -DLLVM_ENABLE_ZSTD:BOOL=OFF -DLLVM_APPEND_VC_REV:BOOL=OFF)

if(WIN32)
    if(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "AMD64")
        list(APPEND LLVM_OPTS -DLLVM_TARGETS_TO_BUILD:STRING=X86)
    else()
        message(FATAL_ERROR "Target CPU is not supported")
    endif()
else()
    if(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "x86_64")
        list(APPEND LLVM_OPTS -DLLVM_TARGETS_TO_BUILD:STRING=X86)
    elseif(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "arm64")
        list(APPEND LLVM_OPTS -DLLVM_TARGETS_TO_BUILD:STRING=AArch64)
    else()
        message(FATAL_ERROR "Target CPU is not supported")
    endif()

    list(APPEND LLVM_OPTS -DCMAKE_BUILD_TYPE:STRING=${CMAKE_BUILD_TYPE})
endif()

if(OB_BUILD_LLVM)
    ExternalProject_Add(llvm
        URL https://github.com/llvm/llvm-project/releases/download/llvmorg-17.0.2/llvm-project-17.0.2.src.tar.xz
        URL_HASH SHA256=351562b14d42fcefcbf00cc1f327680a1062bbbf67a1e1ca6acb64c473b06394
        DOWNLOAD_NO_PROGRESS true
        CMAKE_ARGS -Wno-dev
        CMAKE_CACHE_ARGS ${LLVM_OPTS}
        SOURCE_SUBDIR llvm
        BUILD_ALWAYS ON
        INSTALL_DIR ${CMAKE_SOURCE_DIR}/lib/llvm)
endif()

# Setup Rust target.
set(RUST_OUTPUTS $<IF:$<CONFIG:Debug>,${CMAKE_CURRENT_SOURCE_DIR}/target/debug,${CMAKE_CURRENT_SOURCE_DIR}/target/release>)
set(KERNEL ${RUST_OUTPUTS}/obkrnl${CMAKE_EXECUTABLE_SUFFIX})

if(WIN32)
    set(LIBCORE ${RUST_OUTPUTS}/core.lib)
else()
    set(LIBCORE ${RUST_OUTPUTS}/libcore.a)
endif()

ExternalProject_Add(core
    SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}
    CONFIGURE_COMMAND ""
    BUILD_COMMAND cargo build $<IF:$<CONFIG:Debug>,--profile=dev,--release>
    BUILD_IN_SOURCE ON
    BUILD_ALWAYS ON
    BUILD_BYPRODUCTS ${KERNEL} ${LIBCORE}
    INSTALL_COMMAND ""
    TEST_COMMAND cargo test
    TEST_EXCLUDE_FROM_MAIN ON)

if(OB_BUILD_LLVM)
    add_dependencies(core llvm)
endif()

# Setup application target.
add_executable(obliteration WIN32 MACOSX_BUNDLE
    ansi_escape.cpp
    app_data.cpp
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
        MACOSX_BUNDLE_COPYRIGHT "Copyright © 2023 Obliteration Contributors"
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
