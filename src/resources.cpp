#include "resources.hpp"

#include <QGuiApplication>
#include <QImage>
#include <QPixmap>
#include <QStyleHints>

#include <utility>

QIcon loadIcon(const QString &fileName)
{
    QImage icon(fileName);

    if (QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark) {
        icon.invertPixels();
    }

    return QPixmap::fromImage(std::move(icon));
}
