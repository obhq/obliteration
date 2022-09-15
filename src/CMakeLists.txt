cmake_minimum_required(VERSION 3.12)

project(obliteration)

set(CMAKE_TOOLCHAIN_FILE "${CMAKE_CURRENT_SOURCE_DIR}/../vcpkg/scripts/buildsystems/vcpkg.cmake" CACHE STRING "Vcpkg toolchain file")
set(CMAKE_WINDOWS_EXPORT_ALL_SYMBOLS ON)

add_executable(obliteration main.cpp) 

target_compile_features(obliteration PUBLIC cxx_std_20)

if(WIN32)
	set_target_properties(obliteration PROPERTIES LINK_FLAGS "/DYNAMICBASE:NO /MAP:obliteration.map")
else()
	set_target_properties(obliteration PROPERTIES LINK_FLAGS "-Wl,--image-base=0x100000000000 -Wl,-Map=obliteration.map")
endif()

install(TARGETS obliteration DESTINATION .)
