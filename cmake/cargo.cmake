define_property(TARGET PROPERTY CARGO_TOOLCHAIN)
define_property(TARGET PROPERTY CARGO_TARGET)
define_property(TARGET PROPERTY CARGO_OUTPUTS)

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
                add_library(${crate} STATIC IMPORTED)
            else()
                message(FATAL_ERROR "${kind} crate is not supported")
            endif()

            add_dependencies(${crate} ${build_target})

            # Set target properties.
            set_target_properties(${crate} PROPERTIES
                IMPORTED_CONFIGURATIONS "DEBUG;RELEASE"
                MAP_IMPORTED_CONFIG_MINSIZEREL Release
                MAP_IMPORTED_CONFIG_RELWITHDEBINFO Release
                CARGO_OUTPUTS ${meta_target_directory})

            # Set default Cargo properties.
            set_cargo_properties(${crate}
                ARCHITECTURE ${target_arch}
                VENDOR ${target_vendor}
                OPERATING_SYSTEM ${target_os}
                ENVIRONMENT ${target_env})

            # Add build target.
            set(target "$<TARGET_PROPERTY:${crate},CARGO_TARGET>")
            set(profile "$<IF:$<CONFIG:Debug>,--profile=dev,--release>")

            add_custom_target(${build_target}
                COMMAND cargo build --manifest-path ${manifest} -p ${crate} --target ${target} ${profile})
        endforeach()
    endforeach()
endfunction()

function(set_cargo_properties crate)
    # Parse arguments.
    cmake_parse_arguments(
        PARSE_ARGV 1 arg
        ""
        "ARCHITECTURE;VENDOR;OPERATING_SYSTEM;ENVIRONMENT"
        "")

    # Load current target.
    get_target_property(target ${crate} CARGO_TARGET)

    if(NOT ${target} STREQUAL "target-NOTFOUND")
        string(REGEX MATCHALL "[^\-]+" target ${target})
        list(GET target 0 arch)
        list(GET target 1 vendor)
        list(GET target 2 os)
        list(GET target 3 env)
    endif()

    # Update target.
    if(DEFINED arg_ARCHITECTURE)
        set(arch ${arg_ARCHITECTURE})
    endif()

    if(DEFINED arg_VENDOR)
        set(vendor ${arg_VENDOR})
    endif()

    if(DEFINED arg_OPERATING_SYSTEM)
        set(os ${arg_OPERATING_SYSTEM})
    endif()

    if(DEFINED arg_ENVIRONMENT)
        set(env ${arg_ENVIRONMENT})
    endif()

    # Build triple.
    set(target "${arch}-${vendor}-${os}")

    if(DEFINED env)
        set(target "${target}-${env}")
    endif()

    # Get artifact locations.
    get_target_property(type ${crate} TYPE)
    get_target_property(outputs ${crate} CARGO_OUTPUTS)

    set(debug_outputs "${outputs}/${target}/debug")
    set(release_outputs "${outputs}/${target}/release")

    if(${type} STREQUAL "STATIC_LIBRARY")
        if(WIN32)
            set(debug_artifact "${debug_outputs}/${crate}.lib")
            set(release_artifact "${release_outputs}/${crate}.lib")
        else()
            set(debug_artifact "${debug_outputs}/lib${crate}.a")
            set(release_artifact "${release_outputs}/lib${crate}.a")
        endif()
    else()
        message(FATAL_ERROR "${type} target is not supported")
    endif()

    # Update properties.
    set_target_properties(${crate} PROPERTIES
        CARGO_TARGET ${target}
        IMPORTED_LOCATION_DEBUG ${debug_artifact}
        IMPORTED_LOCATION_RELEASE ${release_artifact})
endfunction()
