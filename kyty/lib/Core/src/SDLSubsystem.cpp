#include "Kyty/Core/SDLSubsystem.h"

#include "Kyty/Core/Common.h"
#include "Kyty/Core/MemoryAlloc.h"

#include <cstring>

// IWYU pragma: no_include <intrin.h>
// IWYU pragma: no_include "SDL_error.h"
// IWYU pragma: no_include "SDL_platform.h"
// IWYU pragma: no_include "SDL_stdinc.h"

#include <SDL2/SDL.h>

namespace Kyty::Core {

KYTY_SUBSYSTEM_UNEXPECTED_SHUTDOWN(SDL) {}

KYTY_SUBSYSTEM_DESTROY(SDL)
{
	SDL_Quit();
}

} // namespace Kyty::Core
