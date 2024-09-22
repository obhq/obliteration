#include "display_settings.hpp"
#include "vulkan.hpp"

#include <QComboBox>
#include <QGridLayout>
#include <QGroupBox>
#include <QMessageBox>
#include <QVBoxLayout>
#ifndef __APPLE__
#include <QVulkanFunctions>
#endif

#include <utility>

#ifdef __APPLE__
DisplaySettings::DisplaySettings(QWidget *parent) :
#else
DisplaySettings::DisplaySettings(QList<VkPhysicalDevice> &&vkDevices, QWidget *parent) :
#endif
    QWidget(parent),
#ifndef __APPLE__
    m_devices(nullptr),
#endif
    m_resolutions(nullptr),
    m_profile(nullptr)
{
    auto layout = new QGridLayout();

#ifdef __APPLE__
    layout->addWidget(buildResolution(), 0, 0);
#else
    layout->addWidget(buildDevice(std::move(vkDevices)), 0, 0);
    layout->addWidget(buildResolution(), 0, 1);
#endif
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

#ifndef __APPLE__
DisplayDevice *DisplaySettings::currentDevice() const
{
    return m_devices->currentData().value<DisplayDevice *>();
}
#endif

#ifndef __APPLE__
QWidget *DisplaySettings::buildDevice(QList<VkPhysicalDevice> &&vkDevices)
{
    // Setup group box.
    auto group = new QGroupBox("Device");
    auto layout = new QVBoxLayout();

    // Setup device list.
    m_devices = new QComboBox();

    for (auto dev : vkDevices) {
        auto data = new DisplayDevice(dev);

        m_devices->addItem(data->name(), QVariant::fromValue(data));
    }

    layout->addWidget(m_devices);

    group->setLayout(layout);

    return group;
}
#endif

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

#ifndef __APPLE__
DisplayDevice::DisplayDevice(VkPhysicalDevice handle) :
    m_handle(handle)
{
    vkFunctions->vkGetPhysicalDeviceProperties(handle, &m_props);
}
#endif
