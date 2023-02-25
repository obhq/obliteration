# CMake modules
include(ExternalProject)

# External dependencies.
find_package(Qt6 COMPONENTS Widgets REQUIRED)
find_package(Threads REQUIRED)

# Setup LLVM target.
if(WIN32)
    set(LLVM_ARGS -DCMAKE_INSTALL_PREFIX:STRING=<INSTALL_DIR>)
else()
    set(LLVM_ARGS -DCMAKE_INSTALL_PREFIX:STRING=<INSTALL_DIR> -DCMAKE_BUILD_TYPE:STRING=${CMAKE_BUILD_TYPE})
endif()

ExternalProject_Add(llvm
    URL https://github.com/llvm/llvm-project/releases/download/llvmorg-15.0.5/llvm-project-15.0.5.src.tar.xz
    URL_HASH SHA256=9c4278a6b8884eb7f4ae7dfe3c8e5445019824885e47cfdf1392563c47316fd6
    DOWNLOAD_EXTRACT_TIMESTAMP false
    DOWNLOAD_NO_PROGRESS true
    CMAKE_CACHE_ARGS ${LLVM_ARGS}
    SOURCE_SUBDIR llvm
    INSTALL_DIR ${CMAKE_SOURCE_DIR}/lib/llvm)

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

add_dependencies(core llvm)

# Setup application target.
add_executable(obliteration WIN32
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
    update_firmware.cpp
    util.cpp)

if(WIN32)
    target_sources(obliteration PRIVATE resources.rc)
endif()

add_dependencies(obliteration core)

set_target_properties(obliteration PROPERTIES AUTOMOC ON AUTORCC ON)

if(WIN32)
    set_target_properties(obliteration PROPERTIES OUTPUT_NAME Obliteration)
endif()

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

install(PROGRAMS ${KERNEL} TYPE BIN)
install(SCRIPT post-install.cmake)
