#include "app_data.hpp"
#include "path.hpp"

#include <QDir>
#include <QStandardPaths>

static QString root()
{
    return QDir::toNativeSeparators(QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation));
}

QString profiles()
{
    auto path = joinPath(root(), "profiles");
    return QString::fromStdString(path);
}
