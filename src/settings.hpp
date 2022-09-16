#pragma once

#include <QString>

bool hasRequiredUserSettings();

// Group registry for QSettings.
namespace SettingGroups {
    extern const QString user;
}
