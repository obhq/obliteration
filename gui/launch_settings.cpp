#include "launch_settings.hpp"
#include "cpu_settings.hpp"
#include "display_settings.hpp"
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
LaunchSettings::LaunchSettings(ProfileList *profiles, QWidget *parent) :
#else
LaunchSettings::LaunchSettings(
    ProfileList *profiles,
    QList<VkPhysicalDevice> &&vkDevices,
    QWidget *parent) :
#endif
    QWidget(parent),
    m_display(nullptr),
    m_cpu(nullptr),
    m_games(nullptr),
    m_profiles(nullptr)
{
    auto layout = new QVBoxLayout();

#ifdef __APPLE__
    layout->addWidget(buildSettings());
#else
    layout->addWidget(buildSettings(std::move(vkDevices)));
#endif
    layout->addLayout(buildActions(profiles));

    setLayout(layout);
}

LaunchSettings::~LaunchSettings()
{
}

Profile *LaunchSettings::currentProfile() const
{
    // Check if profile list is not empty.
    auto index = m_profiles->currentIndex();

    if (index < 0) {
        return nullptr;
    }

    // Get profile.
    auto profiles = reinterpret_cast<ProfileList *>(m_profiles->model());

    return profiles->get(index);
}

#ifndef __APPLE__
DisplayDevice *LaunchSettings::currentDisplayDevice() const
{
    return m_display->currentDevice();
}
#endif

#ifdef __APPLE__
QWidget *LaunchSettings::buildSettings()
#else
QWidget *LaunchSettings::buildSettings(QList<VkPhysicalDevice> &&vkDevices)
#endif
{
    // Tab.
    auto tab = new QTabWidget();
    auto iconSize = tab->iconSize();

    // Display settings.
#ifdef __APPLE__
    m_display = new DisplaySettings();
#else
    m_display = new DisplaySettings(std::move(vkDevices));
#endif

    tab->addTab(m_display, loadIcon(":/resources/monitor.svg", iconSize), "Display");

    // CPU settings.
    m_cpu = new CpuSettings();

    connect(m_cpu, &CpuSettings::debugClicked, [this](const QString &addr) {
        emit startClicked(addr);
    });

    tab->addTab(m_cpu, loadIcon(":/resources/cpu-64-bit.svg", iconSize), "CPU");

    // Game list.
    m_games = new QTableView();
    m_games->setContextMenuPolicy(Qt::CustomContextMenu);
    m_games->setSortingEnabled(true);
    m_games->setWordWrap(false);
    m_games->horizontalHeader()->setSortIndicator(0, Qt::AscendingOrder);
    m_games->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    m_games->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents);
    m_games->verticalHeader()->setSectionResizeMode(QHeaderView::ResizeToContents);

    tab->addTab(m_games, loadIcon(":/resources/view-comfy.svg", iconSize), "Games");

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
    auto save = new QPushButton("Save");

    save->setIcon(loadIcon(":/resources/content-save.svg", save->iconSize()));

    connect(save, &QAbstractButton::clicked, [this]() {
        auto index = m_profiles->currentIndex();

        if (index >= 0) {
            auto profiles = reinterpret_cast<ProfileList *>(m_profiles->model());

            emit saveClicked(profiles->get(index));
        }
    });

    actions->addButton(save, QDialogButtonBox::ApplyRole);

    // Start button.
    auto start = new QPushButton("Start");

    start->setIcon(loadIcon(":/resources/play.svg", start->iconSize()));

    connect(start, &QAbstractButton::clicked, [this]() { emit startClicked({}); });

    actions->addButton(start, QDialogButtonBox::AcceptRole);

    return layout;
}

void LaunchSettings::profileChanged(int index)
{
    assert(index >= 0);

    auto profiles = reinterpret_cast<ProfileList *>(m_profiles->model());
    auto p = profiles->get(index);

    m_display->setProfile(p);
}
