#include "app_data.hpp"
#include "path.hpp"

#include <QDir>
#include <QStandardPaths>

static QString root()
{
    return QDir::toNativeSeparators(QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation));
}

QString kernelDebugDump()
{
    auto path = joinPath(root(), "kernel");
    return QString::fromStdString(path);
}
