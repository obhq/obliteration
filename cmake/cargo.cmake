function(add_cargo)
    # Parse arguments.
    cmake_parse_arguments(
        PARSE_ARGV 0 arg
        ""
        "MANIFEST"
        "")

    if(NOT DEFINED arg_MANIFEST)
        message(FATAL_ERROR "missing MANIFEST option")
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

    # Get metadata.
    execute_process(
        COMMAND cargo metadata --manifest-path ${manifest} --format-version 1
        OUTPUT_VARIABLE meta
        COMMAND_ERROR_IS_FATAL ANY)

    string(JSON meta_target_directory GET ${meta} "target_directory")
    string(JSON meta_packages GET ${meta} "packages")
    string(JSON meta_workspace_members GET ${meta} "workspace_members")

    # Get workspace members.
    string(JSON len LENGTH ${meta_workspace_members})
    math(EXPR len "${len}-1")

    foreach(i RANGE ${len})
        string(JSON id GET ${meta_workspace_members} ${i})
        list(APPEND members ${id})
    endforeach()

    # Get member info.
    string(JSON len LENGTH ${meta_packages})
    math(EXPR len "${len}-1")

    foreach(i RANGE ${len})
        # Skip if not a member.
        string(JSON pkg GET ${meta_packages} ${i})
        string(JSON id GET ${pkg} "id")

        if(NOT id IN_LIST members)
            continue()
        endif()

        # Set variables.
        string(JSON crate GET ${pkg} "name")

        set("CARGO_${crate}_META" ${pkg} PARENT_SCOPE)
        set("CARGO_${crate}_OUTPUTS" ${meta_target_directory} PARENT_SCOPE)
    endforeach()
endfunction()

function(add_crate crate)
    # Check if crate valid.
    if(NOT DEFINED CARGO_${crate}_META)
        message(FATAL_ERROR "crate ${crate} not found")
    endif()

    set(meta ${CARGO_${crate}_META})
    set(outputs ${CARGO_${crate}_OUTPUTS})

    # TODO: Enable CMP0174 to support passing ENVIRONMENT as an empty string.
    cmake_parse_arguments(
        PARSE_ARGV 1 arg
        "LIBRARY"
        "TOOLCHAIN;ARCHITECTURE;VENDOR;OPERATING_SYSTEM"
        "ARGS")

    # Get default target architecture.
    if(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "AMD64" OR ${CMAKE_SYSTEM_PROCESSOR} STREQUAL "x86_64")
        set(target_arch "x86_64")
    elseif(${CMAKE_SYSTEM_PROCESSOR} STREQUAL "arm64" OR ${CMAKE_SYSTEM_PROCESSOR} STREQUAL "aarch64")
        set(target_arch "aarch64")
    else()
        message(FATAL_ERROR "target processor is not supported")
    endif()

    # Get default target OS.
    if(${CMAKE_SYSTEM_NAME} STREQUAL "Darwin")
        set(target_vendor "apple")
        set(target_os "darwin")
    elseif(${CMAKE_SYSTEM_NAME} STREQUAL "Linux")
        set(target_vendor "unknown")
        set(target_os "linux")
        set(target_env "gnu")
    elseif(${CMAKE_SYSTEM_NAME} STREQUAL "Windows")
        set(target_vendor "pc")
        set(target_os "windows")
        set(target_env "msvc")
    else()
        message(FATAL_ERROR "target OS is not supported")
    endif()

    # Override target.
    if(DEFINED arg_ARCHITECTURE)
        set(target_arch ${arg_ARCHITECTURE})
    endif()

    if(DEFINED arg_VENDOR)
        set(target_vendor ${arg_VENDOR})
    endif()

    if(DEFINED arg_OPERATING_SYSTEM)
        set(target_os ${arg_OPERATING_SYSTEM})
    endif()

    if(DEFINED arg_UNPARSED_ARGUMENTS)
        list(FIND arg_UNPARSED_ARGUMENTS "ENVIRONMENT" i)

        if(NOT ${i} STREQUAL "-1")
            math(EXPR i "${i}+1")
            list(GET arg_UNPARSED_ARGUMENTS ${i} target_env)
        endif()
    endif()

    # Build triple.
    set(triple "${target_arch}-${target_vendor}-${target_os}")

    if(NOT "${target_env}" STREQUAL "")
        set(triple "${triple}-${target_env}")
    endif()

    # Get artifact locations.
    set(debug_outputs "${outputs}/${triple}/debug")
    set(release_outputs "${outputs}/${triple}/release")

    if(${target_os} STREQUAL "windows")
        set(bin_suffix ".exe")
    endif()

    # Setup build arguments.
    if(DEFINED arg_TOOLCHAIN)
        set(build_args "+${arg_TOOLCHAIN}")
    endif()

    list(APPEND build_args "build")
    list(APPEND build_args "--target" ${triple})
    list(APPEND build_args "$<IF:$<CONFIG:Debug>,--profile=dev,--release>")
    list(APPEND build_args ${arg_ARGS})

    # Create targets.
    string(JSON manifest GET ${meta} "manifest_path")
    cmake_path(GET manifest PARENT_PATH working_directory)
    string(JSON targets GET ${meta} "targets")
    string(JSON len LENGTH ${targets})
    math(EXPR len "${len}-1")

    foreach(i RANGE ${len})
        # Skip if build script.
        string(JSON target GET ${targets} ${i})
        string(JSON kind GET ${target} "kind" "0")

        if(${kind} STREQUAL "custom-build")
            continue()
        endif()

        # Create imported target.
        set(build_target "${crate}-artifact")

        if(${kind} STREQUAL "staticlib")
            add_library(${crate} STATIC IMPORTED)

            if(arg_LIBRARY)
                list(APPEND build_args "--lib")
            endif()

            if(WIN32)
                set(debug_artifact "${debug_outputs}/${crate}.lib")
                set(release_artifact "${release_outputs}/${crate}.lib")
            else()
                set(debug_artifact "${debug_outputs}/lib${crate}.a")
                set(release_artifact "${release_outputs}/lib${crate}.a")
            endif()
        elseif(arg_LIBRARY)
            continue()
        elseif(${kind} STREQUAL "bin")
            add_executable(${crate} IMPORTED)
            set(debug_artifact "${debug_outputs}/${crate}${bin_suffix}")
            set(release_artifact "${release_outputs}/${crate}${bin_suffix}")
        else()
            message(FATAL_ERROR "${kind} crate is not supported")
        endif()

        add_dependencies(${crate} ${build_target})

        # Set target properties.
        set_target_properties(${crate} PROPERTIES
            IMPORTED_CONFIGURATIONS "DEBUG;RELEASE"
            MAP_IMPORTED_CONFIG_MINSIZEREL Release
            MAP_IMPORTED_CONFIG_RELWITHDEBINFO Release
            IMPORTED_LOCATION_DEBUG ${debug_artifact}
            IMPORTED_LOCATION_RELEASE ${release_artifact})

        # Add build target.
        set(output "$<IF:$<CONFIG:Debug>,${debug_artifact},${release_artifact}>")

        add_custom_target(${build_target}
            COMMAND cargo ${build_args}
            COMMAND ${CMAKE_COMMAND} -E copy -t ${CMAKE_CURRENT_BINARY_DIR} ${output}
            WORKING_DIRECTORY ${working_directory}
            BYPRODUCTS ${output})
    endforeach()
endfunction()
