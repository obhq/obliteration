#pragma once

#include <QDialog>

class Game;
class GameGraphicSettings;
class QDialogButtonBox;
class QTabWidget;

class GameSettingsDialog final : public QDialog {
public:
    GameSettingsDialog(Game *game, QWidget *parent = nullptr);
    ~GameSettingsDialog();

private:
    QTabWidget *m_tab;
    QDialogButtonBox *m_actions;
    GameGraphicSettings *m_graphic;
};
