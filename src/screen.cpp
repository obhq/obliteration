#include "screen.hpp"

Screen::Screen()
{
#ifdef __APPLE__
    setSurfaceType(QSurface::MetalSurface);
#else
    setSurfaceType(QSurface::VulkanSurface);
#endif
}

Screen::~Screen()
{
}
