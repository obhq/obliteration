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
    m_resolutions(nullptr)
{
    auto layout = new QGridLayout();

    layout->addWidget(buildDevice(std::move(vkDevices)), 0, 0);
    layout->setRowStretch(1, 1);

    setLayout(layout);
}

DisplaySettings::~DisplaySettings()
{
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

#ifndef __APPLE__
DisplayDevice::DisplayDevice(VkPhysicalDevice handle) :
    m_handle(handle)
{
    vkFunctions->vkGetPhysicalDeviceProperties(handle, &m_props);
}
#endif
