#include "display_settings.hpp"

#include <QComboBox>
#include <QGridLayout>
#include <QGroupBox>
#include <QMessageBox>
#include <QVBoxLayout>

DisplaySettings::DisplaySettings(QWidget *parent) :
    QWidget(parent),
    m_resolutions(nullptr),
    m_profile(nullptr)
{
    auto layout = new QGridLayout();

    layout->addWidget(buildResolution(), 0, 0);
    layout->setColumnStretch(1, 1);
    layout->setRowStretch(1, 1);

    setLayout(layout);
}

DisplaySettings::~DisplaySettings()
{
}

void DisplaySettings::setProfile(Profile *p)
{
    // Set current profile before update the widgets since the update may trigger some signals.
    m_profile = p;

    // Set resolution.
    auto resolution = profile_display_resolution(p);
    auto i = m_resolutions->findData(resolution);

    if (i < 0) {
        QMessageBox::critical(
            this,
            "Error",
            QString("Unknown display resolution %1.").arg(resolution));
    } else {
        m_resolutions->setCurrentIndex(i);
    }
}

QWidget *DisplaySettings::buildResolution()
{
    // Setup group box.
    auto group = new QGroupBox("Resolution");
    auto layout = new QVBoxLayout();

    // Setup resolution list.
    m_resolutions = new QComboBox();
    m_resolutions->addItem("1280 × 720", DisplayResolution_Hd);
    m_resolutions->addItem("1920 × 1080", DisplayResolution_FullHd);
    m_resolutions->addItem("3840 × 2160", DisplayResolution_UltraHd);

    connect(m_resolutions, &QComboBox::currentIndexChanged, [this](int index) {
        auto value = static_cast<DisplayResolution>(m_resolutions->itemData(index).toInt());

        profile_set_display_resolution(m_profile, value);
    });

    layout->addWidget(m_resolutions);

    group->setLayout(layout);

    return group;
}
