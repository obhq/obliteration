# On Windows qt_generate_deploy_app_script() is broken.
find_program(WINDEPLOYQT windeployqt HINTS ${Qt6_DIR}/bin)

execute_process(COMMAND ${WINDEPLOYQT}
    --release # Use release built binaries.
    --no-quick-import # We don't use Qt Quick.
    --no-translations # Our application is English only so skip Qt translations.
    --no-system-d3d-compiler # --no-quick-import does not implicit enable this.
    --no-opengl-sw # Same here.
    --no-compiler-runtime # We required user to install VC redistribution by themself.
    ${CMAKE_INSTALL_PREFIX}/Obliteration.exe
    COMMAND_ECHO STDOUT)
