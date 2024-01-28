#pragma once

#include <QDialog>

class Game;
class GameGraphicSettings;
class GameSettings;

class GameSettingsDialog final : public QDialog {
public:
    GameSettingsDialog(Game *game, GameSettings *settings, QWidget *parent = nullptr);
    ~GameSettingsDialog();

private slots:
    void save();

private:
    GameSettings *m_settings;
    GameGraphicSettings *m_graphic;
};
