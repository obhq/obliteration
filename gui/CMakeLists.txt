# External dependencies.
find_package(Qt6 REQUIRED COMPONENTS Svg Widgets)
find_package(Threads REQUIRED)

if(WIN32 OR (UNIX AND NOT APPLE))
    find_package(Vulkan REQUIRED)
endif()

if(APPLE)
    find_library(HYPERVISOR Hypervisor REQUIRED)
endif()

# Setup GUI.
add_executable(obliteration WIN32 MACOSX_BUNDLE
    app_data.cpp
    core.cpp
    cpu_settings.cpp
    display_settings.cpp
    game_models.cpp
    initialize_wizard.cpp
    launch_settings.cpp
    main.cpp
    main_window.cpp
    path.cpp
    pkg_extractor.cpp
    pkg_installer.cpp
    profile_models.cpp
    progress_dialog.cpp
    resources.cpp
    resources.qrc
    screen.cpp
    settings.cpp
    system.cpp)

if(WIN32)
    target_sources(obliteration PRIVATE resources.rc vulkan.cpp)
elseif(APPLE)
    target_sources(obliteration PRIVATE resources/obliteration.icns)
elseif(${CMAKE_SYSTEM_NAME} STREQUAL "Linux")
    target_sources(obliteration PRIVATE kvm.cpp vulkan.cpp)
endif()

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

target_link_libraries(obliteration PRIVATE Qt6::Svg Qt6::Widgets)
target_link_libraries(obliteration PRIVATE Threads::Threads)
target_link_libraries(obliteration PRIVATE gui)

if(WIN32 OR (UNIX AND NOT APPLE))
    target_link_libraries(obliteration PRIVATE Vulkan::Vulkan)
endif()

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
    install(IMPORTED_RUNTIME_ARTIFACTS obkrnl DESTINATION obliteration.app/Contents/Resources)
else()
    install(IMPORTED_RUNTIME_ARTIFACTS obkrnl DESTINATION share)
endif()
