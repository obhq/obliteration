# External dependencies.
find_package(Qt6 COMPONENTS Widgets REQUIRED)
find_package(Threads REQUIRED)

if(APPLE)
    find_library(HYPERVISOR Hypervisor REQUIRED)
endif()

# Setup Rust target.
if(WIN32)
    if(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "AMD64")
        set(KERNEL_TARGET x86_64-unknown-none)
    else()
        message(FATAL_ERROR "Target CPU is not supported")
    endif()
else()
    if(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "x86_64")
        set(KERNEL_TARGET x86_64-unknown-none)
    elseif(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "arm64")
        set(KERNEL_TARGET aarch64-unknown-none-softfloat)
    else()
        message(FATAL_ERROR "Target CPU is not supported")
    endif()
endif()

set(KERNEL_OUTPUTS $<IF:$<CONFIG:Debug>,${CMAKE_CURRENT_SOURCE_DIR}/target/${KERNEL_TARGET}/debug,${CMAKE_CURRENT_SOURCE_DIR}/target/${KERNEL_TARGET}/release>)
set(KERNEL ${KERNEL_OUTPUTS}/obkrnl)
set(HOST_OUTPUTS $<IF:$<CONFIG:Debug>,${CMAKE_CURRENT_SOURCE_DIR}/target/debug,${CMAKE_CURRENT_SOURCE_DIR}/target/release>)

if(WIN32)
    set(LIBCORE ${HOST_OUTPUTS}/core.lib)
else()
    set(LIBCORE ${HOST_OUTPUTS}/libcore.a)
endif()

add_custom_target(core
    COMMAND cargo build $<IF:$<CONFIG:Debug>,--profile=dev,--release>
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/core
    BYPRODUCTS ${LIBCORE})

add_custom_target(kernel
    COMMAND cargo build $<IF:$<CONFIG:Debug>,--profile=dev,--release> --target ${KERNEL_TARGET}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/obkrnl
    BYPRODUCTS ${KERNEL})

add_dependencies(core kernel)

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
    launch_settings.cpp
    log_formatter.cpp
    logs_viewer.cpp
    main.cpp
    main_window.cpp
    path.cpp
    pkg_extractor.cpp
    pkg_installer.cpp
    progress_dialog.cpp
    resources.cpp
    resources.qrc
    settings.cpp
    system.cpp)

if(WIN32)
    target_sources(obliteration PRIVATE resources.rc)
elseif(APPLE)
    target_sources(obliteration PRIVATE resources/obliteration.icns)
elseif(${CMAKE_SYSTEM_NAME} STREQUAL "Linux")
    target_sources(obliteration PRIVATE kvm.cpp)
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
    target_link_libraries(obliteration PRIVATE bcrypt imm32 ntdll setupapi userenv version winhvplatform winmm ws2_32)
elseif(APPLE)
    target_link_libraries(obliteration PRIVATE ${HYPERVISOR})
else()
    target_link_libraries(obliteration PRIVATE ${CMAKE_DL_LIBS})
endif()

if(APPLE)
    add_custom_command(TARGET obliteration POST_BUILD
        COMMAND codesign -s - --entitlements ${CMAKE_SOURCE_DIR}/entitlements.plist obliteration.app)
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
    install(FILES ${KERNEL} DESTINATION obliteration.app/Contents/Resources)
else()
    install(FILES ${KERNEL} TYPE DATA)
endif()
