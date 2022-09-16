# On Windows we need to copy Qt runtime DLLs to the application directory.
if(WIN32)
    find_program(WINDEPLOYQT windeployqt HINTS ${Qt5_DIR}/bin)
    execute_process(COMMAND ${WINDEPLOYQT} --release --no-translations --no-compiler-runtime ${CMAKE_INSTALL_PREFIX}/obliteration.exe
        COMMAND_ECHO STDOUT)
endif()
