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

    # Generate Cargo.lock.
    cmake_path(REPLACE_FILENAME manifest Cargo.lock OUTPUT_VARIABLE cargo_lock)

    if(NOT EXISTS ${cargo_lock})
        message(STATUS "Generating Cargo.lock")
        execute_process(
            COMMAND cargo generate-lockfile --manifest-path ${manifest}
            COMMAND_ERROR_IS_FATAL ANY)
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

        set(debug_outputs ${meta_target_directory}/debug)
        set(release_outputs ${meta_target_directory}/release)

        # Create targets.
        string(JSON targets GET ${pkg} "targets")
        string(JSON len LENGTH ${targets})
        math(EXPR len "${len}-1")

        foreach(i RANGE ${len})
            # Skip if build script.
            string(JSON target GET ${targets} ${i})
            string(JSON kind GET ${target} "kind" "0")

            if(${kind} STREQUAL "custom-build")
                continue()
            elseif(TARGET ${crate})
                message(FATAL_ERROR "multiple crate types is not supported")
            endif()

            # Create imported target.
            set(build_target "${crate}-artifact")

            if(${kind} STREQUAL "staticlib")
                if(WIN32)
                    set(debug_artifact ${debug_outputs}/${crate}.lib)
                    set(release_artifact ${release_outputs}/${crate}.lib)
                else()
                    set(debug_artifact ${debug_outputs}/lib${crate}.a)
                    set(release_artifact ${release_outputs}/lib${crate}.a)
                endif()

                add_library(${crate} STATIC IMPORTED)
                add_dependencies(${crate} ${build_target})
                set_target_properties(${crate} PROPERTIES
                    IMPORTED_CONFIGURATIONS "DEBUG;RELEASE"
                    IMPORTED_LOCATION_DEBUG ${debug_artifact}
                    IMPORTED_LOCATION_RELEASE ${release_artifact}
                    MAP_IMPORTED_CONFIG_MINSIZEREL Release
                    MAP_IMPORTED_CONFIG_RELWITHDEBINFO Release)
            else()
                message(FATAL_ERROR "${kind} crate is not supported")
            endif()

            # Add build target.
            add_custom_target(${build_target}
                COMMAND cargo build --manifest-path ${manifest} -p ${crate} $<IF:$<CONFIG:Debug>,--profile=dev,--release>
                BYPRODUCTS $<IF:$<CONFIG:Debug>,${debug_artifact},${release_artifact}>)
        endforeach()
    endforeach()
endfunction()
