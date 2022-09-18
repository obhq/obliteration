#include "game_settings_dialog.hpp"

GameSettingsDialog::GameSettingsDialog(Game *game, QWidget *parent) :
    QDialog(parent),
    m_game(game)
{
}

GameSettingsDialog::~GameSettingsDialog()
{
}
