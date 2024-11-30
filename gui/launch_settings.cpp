#include "launch_settings.hpp"
#include "display_settings.hpp"
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
LaunchSettings::LaunchSettings(QWidget *parent) :
#else
LaunchSettings::LaunchSettings(
    QList<VkPhysicalDevice> &&vkDevices,
    QWidget *parent) :
#endif
    QWidget(parent),
    m_display(nullptr),
    m_profiles(nullptr)
{
    auto layout = new QVBoxLayout();

#ifdef __APPLE__
    layout->addWidget(buildSettings());
#else
    layout->addWidget(buildSettings(std::move(vkDevices)));
#endif
    layout->addLayout(buildActions());

    setLayout(layout);
}

LaunchSettings::~LaunchSettings()
{
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
    auto save = new QPushButton("Save");

    save->setIcon(loadIcon(":/resources/content-save.svg", save->iconSize()));

    actions->addButton(save, QDialogButtonBox::ApplyRole);

    // Start button.
    auto start = new QPushButton("Start");

    start->setIcon(loadIcon(":/resources/play.svg", start->iconSize()));

    connect(start, &QAbstractButton::clicked, [this]() { emit startClicked({}); });

    actions->addButton(start, QDialogButtonBox::AcceptRole);

    return layout;
}
