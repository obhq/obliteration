# External dependencies.
find_package(Qt5 COMPONENTS Widgets REQUIRED)

# Setup application target.
add_executable(obliteration WIN32
    game_models.cpp
    initialize_dialog.cpp
    main.cpp
    main_window.cpp
    settings.cpp)

set_property(TARGET obliteration PROPERTY AUTOMOC ON)
set_property(TARGET obliteration PROPERTY AUTORCC ON)

target_compile_features(obliteration PRIVATE cxx_std_17)

target_link_libraries(obliteration PRIVATE Qt5::Widgets)

# Setup installation.
if(WIN32)
    install(TARGETS obliteration DESTINATION .)
else()
    install(TARGETS obliteration DESTINATION bin)
endif()

install(SCRIPT post-install.cmake)
