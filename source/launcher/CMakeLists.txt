
find_package(Qt5 COMPONENTS Widgets LinguistTools REQUIRED)

set(CMAKE_AUTOUIC_SEARCH_PATHS forms)

file(GLOB launcher_src
	"include/*.h"
	"src/*.cpp"
)

file(GLOB launcher_forms
	"forms/*.ui"
)

file(GLOB launcher_res
	"res/*.qrc"
)

set(launcher_ts "ts/launcher_ru_RU.ts")

add_executable(launcher WIN32
	${launcher_src}
	${launcher_forms}
	${launcher_res}
	${launcher_ts}
)

if(WIN32)
	find_program(QT_WINDEPLOYQT NAMES windeployqt PATHS "${Qt5_DIR}/../../../bin")

	if(NOT QT_WINDEPLOYQT)
		message(FATAL_ERROR "Could not find windeployqt")
	endif()

	install(CODE "
			execute_process(
				COMMAND ${QT_WINDEPLOYQT} \"\${CMAKE_INSTALL_PREFIX}/${launcher_name}.exe\"
			--no-svg
			--no-angle
			--no-opengl
			--no-opengl-sw
			--no-system-d3d-compiler
					--no-translations
			)
	")
endif()



qt5_create_translation(QM_FILES ${launcher_src} ${launcher_forms} ${launcher_ts} OPTIONS -I ${CMAKE_SOURCE_DIR}/launcher/include)
