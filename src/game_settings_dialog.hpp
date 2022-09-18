#pragma once

#include <QDialog>

class Game;

class GameSettingsDialog final : public QDialog {
public:
    GameSettingsDialog(Game *game, QWidget *parent = nullptr);
    ~GameSettingsDialog();

private:
    Game *m_game;
};
