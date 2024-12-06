#include "launch_settings.hpp"
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

LaunchSettings::LaunchSettings(QWidget *parent) :
    QWidget(parent),
    m_profiles(nullptr)
{
    auto layout = new QVBoxLayout();

    layout->addWidget(buildSettings());
    layout->addLayout(buildActions());

    setLayout(layout);
}

LaunchSettings::~LaunchSettings()
{
}

QWidget *LaunchSettings::buildSettings()
{
    // Tab.
    auto tab = new QTabWidget();

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
