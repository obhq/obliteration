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

bool Screen::event(QEvent *ev)
{
    if (ev->type() == QEvent::UpdateRequest) {
        emit updateRequestReceived();
    }

    return QWindow::event(ev);
}
