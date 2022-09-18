#pragma once

class GameSettings final {
public:
    GameSettings();
    ~GameSettings();

public:
    enum Resolution {
        Hd, // 1280x720
        FullHd, // 1920x1080
    };

public:
    Resolution resolution() const { return m_resolution; }
    void setResolution(Resolution v) { m_resolution = v; }

private:
    Resolution m_resolution;
};
