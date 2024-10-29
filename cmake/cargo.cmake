function(add_cargo)
    # Parse arguments.
    cmake_parse_arguments(
        PARSE_ARGV 0 arg
        ""
        "MANIFEST"
        "CRATES")

    if(NOT DEFINED arg_MANIFEST)
        message(FATAL_ERROR "missing MANIFEST option")
    endif()

    if(NOT DEFINED arg_CRATES)
        message(FATAL_ERROR "missing CRATES option")
    endif()

    # Get absolute path to Cargo manifest.
    cmake_path(IS_RELATIVE arg_MANIFEST relative)

    if(relative)
        set(manifest ${CMAKE_CURRENT_SOURCE_DIR}/${arg_MANIFEST})
    else()
        set(manifest ${arg_MANIFEST})
    endif()


    # Get crate ID.
    foreach(crate ${arg_CRATES})
        execute_process(
            COMMAND cargo pkgid --manifest-path ${manifest} -p ${crate}
            OUTPUT_VARIABLE pkgid
            COMMAND_ERROR_IS_FATAL ANY)
        string(STRIP ${pkgid} pkgid)
        list(APPEND ids ${pkgid})
    endforeach()

    # Get metadata.
    execute_process(
        COMMAND cargo metadata --manifest-path ${manifest} --format-version 1
        OUTPUT_VARIABLE meta
        COMMAND_ERROR_IS_FATAL ANY)

    string(JSON meta_target_directory GET ${meta} "target_directory")
    string(JSON meta_packages GET ${meta} "packages")
    string(JSON len LENGTH ${meta_packages})

    if(${CMAKE_BUILD_TYPE} STREQUAL "Debug")
        set(build_type "debug")
        set(profile "dev")
    else()
        set(build_type "release")
        set(profile "release")
    endif()

    # Create CMake targets.
    math(EXPR len "${len}-1")

    foreach(i RANGE ${len})
        # Skip if not target crate.
        string(JSON pkg GET ${meta_packages} ${i})
        string(JSON id GET ${pkg} "id")
        list(FIND ids ${id} i)

        if(${i} STREQUAL "-1")
            continue()
        endif()

        list(GET arg_CRATES ${i} crate)

        # Create targets.
        string(JSON targets GET ${pkg} "targets")
        string(JSON len LENGTH ${targets})
        math(EXPR len "${len}-1")

        foreach(i RANGE ${len})
            # Create imported target.
            string(JSON target GET ${targets} ${i})
            string(JSON kind GET ${target} "kind" "0")
            set(build_target "${crate}-artifact")

            if(${kind} STREQUAL "staticlib")
                if(WIN32)
                    set(output ${meta_target_directory}/${build_type}/${crate}.lib)
                else()
                    set(output ${meta_target_directory}/${build_type}/lib${crate}.a)
                endif()

                add_library(${crate} STATIC IMPORTED)
                set_target_properties(${crate} PROPERTIES IMPORTED_LOCATION ${output})
                add_dependencies(${crate} ${build_target})
            elseif(${kind} STREQUAL "custom-build")
                continue()
            else()
                message(FATAL_ERROR "${kind} crate is not supported")
            endif()

            # Add build target.
            add_custom_target(${build_target}
                COMMAND cargo build --manifest-path ${manifest} -p ${crate} --profile ${profile}
                BYPRODUCTS ${output})
        endforeach()
    endforeach()
endfunction()
