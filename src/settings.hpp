#pragma once

#include <QString>

QString readGamesDirectorySetting();
void writeGamesDirectorySetting(const QString &v);

// Group registry for QSettings.
namespace SettingGroups {
    extern const QString user;
    extern const QString mainWindow;
}
