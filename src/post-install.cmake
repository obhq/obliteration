# On Windows we need to copy Qt runtime DLLs to the application directory.
if(WIN32)
    # https://doc.qt.io/Qt-6/windows-deployment.html
    find_program(WINDEPLOYQT windeployqt HINTS ${Qt6_DIR}/bin)
    execute_process(COMMAND ${WINDEPLOYQT}
        --release # Use release built binaries.
        --no-quick-import # We don't use Qt Quick.
        --no-translations # Our application is English only so skip Qt translations.
        --no-compiler-runtime # We required user to install VC redistribution by themself.
        ${CMAKE_INSTALL_PREFIX}/obliteration.exe
        COMMAND_ECHO STDOUT)
endif()
