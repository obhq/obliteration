#pragma once

#include <QIcon>

class QSize;

/// Only SVG file is supported.
QIcon loadIcon(const QString &fileName, const QSize &size);
