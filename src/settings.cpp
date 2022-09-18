#include "settings.hpp"

#include <QSettings>

// Keys for user settings.
namespace UserSettings {
    static const QString gamesDirectory("gamesDirectory");
}

#define scope(name) QSettings s; s.beginGroup(name)

QString readGamesDirectorySetting()
{
    scope(SettingGroups::user);

    auto v = s.value(UserSettings::gamesDirectory);

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
