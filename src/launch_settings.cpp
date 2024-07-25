#include "launch_settings.hpp"
#include "display_settings.hpp"
#include "game_models.hpp"
#include "game_settings.hpp"
#include "game_settings_dialog.hpp"
#include "resources.hpp"

#include <QComboBox>
#include <QDesktopServices>
#include <QDialogButtonBox>
#include <QHeaderView>
#include <QHBoxLayout>
#include <QMenu>
#include <QPushButton>
#include <QTableView>
#include <QTabWidget>
#include <QUrl>
#include <QVBoxLayout>

LaunchSettings::LaunchSettings(GameListModel *games, QWidget *parent) :
    QWidget(parent),
    m_display(nullptr),
    m_games(nullptr),
    m_profiles(nullptr)
{
    auto layout = new QVBoxLayout();

    layout->addWidget(buildSettings(games));
    layout->addLayout(buildActions());

    setLayout(layout);
}

LaunchSettings::~LaunchSettings()
{
}

QWidget *LaunchSettings::buildSettings(GameListModel *games)
{
    // Tab.
    auto tab = new QTabWidget();

    // Display settings.
    m_display = new DisplaySettings();

    tab->addTab(m_display, loadIcon(":/resources/monitor.svg"), "Display");

    // Game list.
    m_games = new QTableView();
    m_games->setContextMenuPolicy(Qt::CustomContextMenu);
    m_games->setSortingEnabled(true);
    m_games->setWordWrap(false);
    m_games->setModel(games);
    m_games->horizontalHeader()->setSortIndicator(0, Qt::AscendingOrder);
    m_games->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    m_games->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    m_games->verticalHeader()->setSectionResizeMode(QHeaderView::ResizeToContents);

    connect(m_games, &QWidget::customContextMenuRequested, this, &LaunchSettings::requestGamesContextMenu);

    tab->addTab(m_games, loadIcon(":/resources/view-comfy.svg"), "Games");

    return tab;
}

QLayout *LaunchSettings::buildActions()
{
    auto layout = new QHBoxLayout();

    // Profile list.
    m_profiles = new QComboBox();

    layout->addWidget(m_profiles, 1);

    // Actions bar.
    auto actions = new QDialogButtonBox();

    layout->addWidget(actions);

    // Save button.
    auto save = actions->addButton("Save", QDialogButtonBox::ApplyRole);

    // Start button.
    auto start = actions->addButton("Start", QDialogButtonBox::AcceptRole);

    connect(start, &QAbstractButton::clicked, [this]() { emit startClicked(); });

    return layout;
}

void LaunchSettings::requestGamesContextMenu(const QPoint &pos)
{
    // Get item index.
    auto index = m_games->indexAt(pos);

    if (!index.isValid()) {
        return;
    }

    auto model = reinterpret_cast<GameListModel *>(m_games->model());
    auto game = model->get(index.row());

    // Setup menu.
    QMenu menu(this);
    QAction openFolder(loadIcon(":/resources/folder-open-outline.svg"), "Open &Folder", this);
    QAction settings(loadIcon(":/resources/cog-outline.svg"), "&Settings", this);

    menu.addAction(&openFolder);
    menu.addAction(&settings);

    // Show menu.
    auto selected = menu.exec(m_games->viewport()->mapToGlobal(pos));

    if (!selected) {
        return;
    }

    if (selected == &openFolder) {
        QDesktopServices::openUrl(QUrl::fromLocalFile(game->directory()));
    } else if (selected == &settings) {
        // Load settings then show a dialog to edit.
        auto settings = GameSettings::load(game);
        GameSettingsDialog dialog(game, settings.get(), this);

        dialog.exec();
    }
}
