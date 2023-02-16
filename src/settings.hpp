#pragma once

#include <QString>

bool hasRequiredUserSettings();

bool hasSystemDirectorySetting();
QString readSystemDirectorySetting();
void writeSystemDirectorySetting(const QString &v);

bool hasGamesDirectorySetting();
QString readGamesDirectorySetting();
void writeGamesDirectorySetting(const QString &v);

// Group registry for QSettings.
namespace SettingGroups {
    extern const QString user;
    extern const QString mainWindow;
}
