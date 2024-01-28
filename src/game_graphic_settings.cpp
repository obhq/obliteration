#include "game_graphic_settings.hpp"

#include <QComboBox>
#include <QGridLayout>
#include <QGroupBox>
#include <QLabel>
#include <QSizePolicy>
#include <QVBoxLayout>

GameGraphicSettings::GameGraphicSettings(QWidget *parent) :
    QWidget(parent),
    m_mode(nullptr)
{
    auto layout = new QVBoxLayout();

    layout->addWidget(setupModeWidget());
    layout->addStretch(1);

    setLayout(layout);
}

GameGraphicSettings::~GameGraphicSettings()
{
}

QGroupBox *GameGraphicSettings::setupModeWidget()
{
    auto group = new QGroupBox("Mode");
    auto layout = new QGridLayout();

    // Label.
    auto label = new QLabel("&Mode:");
    layout->addWidget(label, 0, 0);

    // Selection.
    m_mode = new QComboBox();
    m_mode->addItem("PlayStation 4");
    m_mode->addItem("PlayStation 4 Pro");

    label->setBuddy(m_mode);
    layout->addWidget(m_mode, 0, 1);
    layout->setColumnStretch(1, 1);

    // Description.
    auto desc = new QLabel(
        R"(Mode of the PS4 to run this game. Pro mode will use more resources so if you have any )"
        R"(performance problems try standard mode instead.)");

    desc->setWordWrap(true);

    layout->addWidget(desc, 1, 0, 1, 2);

    group->setSizePolicy(QSizePolicy::MinimumExpanding, QSizePolicy::Minimum);
    group->setLayout(layout);

    return group;
}
