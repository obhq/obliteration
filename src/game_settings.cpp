#include "game_settings.hpp"
#include "game_models.hpp"
#include "settings.hpp"

#include <QSettings>

GameSettings::GameSettings() :
    m_mode(Mode::Standard),
    m_resolution(Resolution::Hd)
{
}

GameSettings::~GameSettings()
{
}

QScopedPointer<GameSettings> GameSettings::load(Game *game)
{
    auto m = new GameSettings();
    QSettings s;

    // Set QSettings group.
    s.beginGroup(SettingGroups::games);
    s.beginGroup(game->id());

    // Load settings.
    if (auto v = s.value("mode"); !v.isNull()) {
        m->m_mode = static_cast<Mode>(v.toInt());
    }

    return QScopedPointer<GameSettings>(m);
}
