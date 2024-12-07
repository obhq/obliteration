# External dependencies.
find_package(Qt6 REQUIRED COMPONENTS Svg Widgets)
find_package(Threads REQUIRED)

if(WIN32 OR (UNIX AND NOT APPLE))
    find_package(Vulkan REQUIRED)
endif()

# Setup GUI.
add_executable(obliteration WIN32 MACOSX_BUNDLE
    main.cpp
    main_window.cpp
    path.cpp
    resources.cpp
    resources.qrc
    settings.cpp)

if(WIN32)
    target_sources(obliteration PRIVATE vulkan.cpp)
elseif(${CMAKE_SYSTEM_NAME} STREQUAL "Linux")
    target_sources(obliteration PRIVATE vulkan.cpp)
endif()

set_target_properties(obliteration PROPERTIES AUTOMOC ON AUTORCC ON)

target_compile_features(obliteration PRIVATE cxx_std_17)

target_link_libraries(obliteration PRIVATE Qt6::Svg Qt6::Widgets)
target_link_libraries(obliteration PRIVATE Threads::Threads)

if(WIN32 OR (UNIX AND NOT APPLE))
    target_link_libraries(obliteration PRIVATE Vulkan::Vulkan)
endif()

if(WIN32)
    target_link_libraries(obliteration PRIVATE bcrypt imm32 ntdll setupapi userenv version winhvplatform winmm ws2_32)
else()
    target_link_libraries(obliteration PRIVATE ${CMAKE_DL_LIBS})
endif()
