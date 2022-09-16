#include "settings.hpp"

#include <QSettings>

// Keys for user settings.
namespace UserSettings {
    static const QString gamesDirectory("gamesDirectory");
}

bool hasRequiredUserSettings()
{
    QSettings s;

    s.beginGroup(SettingGroups::user);

    return s.contains(UserSettings::gamesDirectory);
}

namespace SettingGroups {
    const QString user("user");
}
