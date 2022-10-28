#include "settings.hpp"

#include <QSettings>

// Keys for user settings.
namespace UserSettings {
    static const QString systemDirectory("systemDirectory");
    static const QString gamesDirectory("gamesDirectory");
}

#define scope(name) QSettings s; s.beginGroup(name)

bool hasRequiredUserSettings()
{
    scope(SettingGroups::user);

    return s.contains(UserSettings::systemDirectory) && s.contains(UserSettings::gamesDirectory);
}

QString readSystemDirectorySetting()
{
    scope(SettingGroups::user);

    auto v = s.value(UserSettings::systemDirectory);
    if (v.isEmpty()) {
        v = "C:\ProgramData\Obliteration\SystemData\";
    }

    return v.isNull() ? QString() : v.toString();
}

void writeSystemDirectorySetting(const QString &v)
{
    scope(SettingGroups::user);

    s.setValue(UserSettings::systemDirectory, v);
}

QString readGamesDirectorySetting()
{
    scope(SettingGroups::user);

    auto v = s.value(UserSettings::gamesDirectory);
    
    if (v.isEmpty()) {
        v = "C:\ProgramData\Obliteration\GameData\";
    }

    return v.isNull() ? QString() : v.toString();
}

void writeGamesDirectorySetting(const QString &v)
{
    scope(SettingGroups::user);

    s.setValue(UserSettings::gamesDirectory, v);
}

namespace SettingGroups {
    const QString user("user");
    const QString mainWindow("mainWindow");
}
