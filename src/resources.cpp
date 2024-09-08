#include "resources.hpp"

#include <QGuiApplication>
#include <QImage>
#include <QPainter>
#include <QPixmap>
#include <QStyleHints>
#include <QSvgRenderer>

#include <utility>

QIcon loadIcon(const QString &fileName, const QSize &size)
{
    // Prepare to render the icon. We use the highest pixel ratio here so the icon will look sharp
    // on any screen if the user have multiple monitors.
    QSvgRenderer renderer(fileName);
    QImage icon(size * qGuiApp->devicePixelRatio(), QImage::Format_ARGB32);

    icon.fill(0);

    // Render.
    QPainter painter(&icon);

    renderer.render(&painter);

    if (QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark) {
        icon.invertPixels();
    }

    return QPixmap::fromImage(std::move(icon));
}
