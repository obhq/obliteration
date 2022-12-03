cmake_minimum_required(VERSION 3.16)

project(obliteration)

# Set warning level to highest. Thie will propagate to sub-directories too.
if(WIN32)
    add_compile_options(/W4)
else()
    add_compile_options(-Wall -Wextra)
endif()

add_subdirectory(src)
