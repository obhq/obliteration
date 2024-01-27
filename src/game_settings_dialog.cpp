#include "game_settings_dialog.hpp"
#include "game_graphic_settings.hpp"
#include "game_models.hpp"

#include <QDialogButtonBox>
#include <QTabBar>
#include <QTabWidget>
#include <QVBoxLayout>

GameSettingsDialog::GameSettingsDialog(Game *game, QWidget *parent) :
    QDialog(parent),
    m_tab(nullptr),
    m_actions(nullptr),
    m_graphic(nullptr)
{
    auto layout = new QVBoxLayout(this);

    // Main tab.
    m_tab = new QTabWidget();
    layout->addWidget(m_tab);

    // Actions bar.
    m_actions = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel);
    layout->addWidget(m_actions);

    // Graphic tab.
    m_graphic = new GameGraphicSettings();
    m_tab->addTab(m_graphic, "Graphic");

    setWindowTitle(game->name());
}

GameSettingsDialog::~GameSettingsDialog()
{
}
