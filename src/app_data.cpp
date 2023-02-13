#include "app_data.hpp"
#include "path.hpp"

#include <QStandardPaths>

static QString root()
{
    return QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
}

QString kernelDebugDump()
{
    auto path = joinPath(root(), "kernel");
    return QString::fromStdString(path);
}
