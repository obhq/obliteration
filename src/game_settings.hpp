#pragma once

#include <QScopedPointer>

class Game;

class GameSettings final {
public:
    GameSettings();
    ~GameSettings();

public:
    static QScopedPointer<GameSettings> load(Game *game);

public:
    enum Mode {
        Standard = 0,
        Pro = 1,
    };

    enum Resolution {
        Hd = 0, // 1280x720
        FullHd = 1, // 1920x1080
    };

public:
    Mode mode() const { return m_mode; }
    void setMode(Mode v) { m_mode = v; }

    Resolution resolution() const { return m_resolution; }
    void setResolution(Resolution v) { m_resolution = v; }

private:
    Mode m_mode;
    Resolution m_resolution;
};
