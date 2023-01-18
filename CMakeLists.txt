cmake_minimum_required(VERSION 3.21)

project(obliteration)

# Set warning level to highest. Thie will propagate to sub-directories too.
if(WIN32)
    add_compile_options(/W4)
    set(CMAKE_PREFIX_PATH "C:/Qt/6.2.4/msvc2019_64")
else()
    add_compile_options(-Wall -Wextra)
endif()

add_subdirectory(src)
