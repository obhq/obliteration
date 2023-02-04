# CMake modules
include(ExternalProject)

# External dependencies.
find_package(Qt6 COMPONENTS Widgets REQUIRED)
find_package(Threads REQUIRED)

set(CMAKE_INCLUDE_CURRENT_DIR ON)
set(prefix "${PROJECT_NAME}.app/Contents")
set(INSTALL_RUNTIME_DIR "${prefix}/MacOS")
set(INSTALL_CMAKE_DIR "${prefix}/Resources")

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

# Setup application target.
add_executable(obliteration MACOSX_BUNDLE
    app_data.cpp
    game_models.cpp
    game_settings.cpp
    game_settings_dialog.cpp
    initialize_dialog.cpp
    main.cpp
    main_window.cpp
    path.cpp
    progress_dialog.cpp
    resources.qrc
    settings.cpp
    system.cpp
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

# install(PROGRAMS ${KERNEL} TYPE BIN)
install(TARGETS obliteration
    BUNDLE DESTINATION . COMPONENT Runtime
    RUNTIME DESTINATION bin COMPONENT Runtime
    )

set(APPS "\${CMAKE_INSTALL_PREFIX}/obliteration.app")
# install(SCRIPT post-install.cmake)

# Build MacOS package
# set_target_properties(obliteration PROPERTIES
#     MACOSX_BUNDLE True
#     MACOSX_BUNDLE_GUI_IDENTIFIER com.github.obhq.obliteration
#     MACOSX_BUNDLE_BUNDLE_NAME obliteration
#     MACOSX_BUNDLE_BUNDLE_VERSION "0.1"
#     MACOSX_BUNDLE_SHORT_VERSION_STRING "0.1"
#     MACOSX_BUNDLE_INFO_PLIST ${CMAKE_SOURCE_DIR}/cmake/customtemplate.plist.in
# )


# Directories to look for dependencies
set(DIRS ${CMAKE_BINARY_DIR})

install(CODE "include(BundleUtilities)
    fixup_bundle(\"${APPS}\" \"\" \"${DIRS}\")")

set(CPACK_GENERATOR "DRAGNDROP")
include(CPack)
