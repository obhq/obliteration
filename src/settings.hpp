#pragma once

#include <QString>

bool hasRequiredUserSettings();

QString readSystemDirectorySetting();
void writeSystemDirectorySetting(const QString &v);

QString readGamesDirectorySetting();
void writeGamesDirectorySetting(const QString &v);

// Group registry for QSettings.
namespace SettingGroups {
    extern const QString user;
    extern const QString mainWindow;
}
