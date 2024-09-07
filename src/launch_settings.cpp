#include "launch_settings.hpp"
#include "display_settings.hpp"
#include "game_models.hpp"
#include "game_settings.hpp"
#include "game_settings_dialog.hpp"
#include "profile_models.hpp"
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

#include <utility>

#ifdef __APPLE__
LaunchSettings::LaunchSettings(ProfileList *profiles, GameListModel *games, QWidget *parent) :
#else
LaunchSettings::LaunchSettings(
    ProfileList *profiles,
    GameListModel *games,
    QList<VkPhysicalDevice> &&vkDevices,
    QWidget *parent) :
#endif
    QWidget(parent),
    m_display(nullptr),
    m_games(nullptr),
    m_profiles(nullptr)
{
    auto layout = new QVBoxLayout();

    layout->addWidget(buildSettings(games, std::move(vkDevices)));
    layout->addLayout(buildActions(profiles));

    setLayout(layout);
}

LaunchSettings::~LaunchSettings()
{
}

#ifdef __APPLE__
QWidget *LaunchSettings::buildSettings(GameListModel *games)
#else
QWidget *LaunchSettings::buildSettings(GameListModel *games, QList<VkPhysicalDevice> &&vkDevices)
#endif
{
    // Tab.
    auto tab = new QTabWidget();

    // Display settings.
    m_display = new DisplaySettings(std::move(vkDevices));

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

QLayout *LaunchSettings::buildActions(ProfileList *profiles)
{
    auto layout = new QHBoxLayout();

    // Profile list.
    m_profiles = new QComboBox();
    m_profiles->setModel(profiles);

    connect(m_profiles, &QComboBox::currentIndexChanged, this, &LaunchSettings::profileChanged);

    layout->addWidget(m_profiles, 1);

    // Actions bar.
    auto actions = new QDialogButtonBox();

    layout->addWidget(actions);

    // Save button.
    auto save = new QPushButton(loadIcon(":/resources/content-save.svg"), "Save");

    connect(save, &QAbstractButton::clicked, [this]() {
        auto index = m_profiles->currentIndex();

        if (index >= 0) {
            auto profiles = reinterpret_cast<ProfileList *>(m_profiles->model());

            emit saveClicked(profiles->get(index));
        }
    });

    actions->addButton(save, QDialogButtonBox::ApplyRole);

    // Start button.
    auto start = new QPushButton(loadIcon(":/resources/play.svg"), "Start");

    connect(start, &QAbstractButton::clicked, [this]() { emit startClicked(); });

    actions->addButton(start, QDialogButtonBox::AcceptRole);

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

void LaunchSettings::profileChanged(int index)
{
    assert(index >= 0);

    auto profiles = reinterpret_cast<ProfileList *>(m_profiles->model());
    auto p = profiles->get(index);

    m_display->setProfile(p);
}
