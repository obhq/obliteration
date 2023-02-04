cmake_minimum_required(VERSION 3.21)

project(obliteration)

# Set warning level to highest. Thie will propagate to sub-directories too.
if(WIN32)
    add_compile_options(/W4)
else()
    add_compile_options(-Wall -Wextra)
endif()

add_subdirectory(src)

if(UNIX AND NOT APPLE)
    set(CPACK_PACKAGE_VERSION_MAJOR 0)
    set(CPACK_PACKAGE_VERSION_MINOR 1)
    set(CPACK_GENERATOR "DEB")
    set(CPACK_PACKAGE_NAME "obliteration")
    SET(CPACK_PACKAGING_INSTALL_PREFIX "/opt/obliteration")
    set(CPACK_DEBIAN_PACKAGE_MAINTAINER "yisonPylkita <yisonPylkita@gmail.com>")
    set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "PlayStation 4 emulator")
    set(CPACK_DEBIAN_PACKAGE_DEPENDS "qt6-base-dev")
    include(CPack)
endif()
