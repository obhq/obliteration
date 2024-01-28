#include "game_settings_dialog.hpp"
#include "game_graphic_settings.hpp"
#include "game_models.hpp"

#include <QDialogButtonBox>
#include <QTabBar>
#include <QTabWidget>
#include <QVBoxLayout>

GameSettingsDialog::GameSettingsDialog(Game *game, GameSettings *settings, QWidget *parent) :
    QDialog(parent),
    m_settings(settings),
    m_graphic(nullptr)
{
    auto layout = new QVBoxLayout(this);

    // Main tab.
    auto tab = new QTabWidget();
    layout->addWidget(tab);

    // Actions bar.
    auto actions = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel);

    connect(actions, &QDialogButtonBox::accepted, this, &GameSettingsDialog::save);
    connect(actions, &QDialogButtonBox::rejected, this, &QDialog::reject);

    layout->addWidget(actions);

    // Graphic tab.
    m_graphic = new GameGraphicSettings(settings);
    tab->addTab(m_graphic, "Graphic");

    setWindowTitle(game->name());
}

GameSettingsDialog::~GameSettingsDialog()
{
}

void GameSettingsDialog::save()
{
}
