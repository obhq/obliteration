#pragma once

#include "core.h"

#ifndef __APPLE__
#include <QVulkanInstance>
#endif
#include <QWidget>

class QComboBox;

class DisplaySettings final : public QWidget {
public:
#ifdef __APPLE__
    DisplaySettings(QWidget *parent = nullptr);
#else
    DisplaySettings(QList<VkPhysicalDevice> &&vkDevices, QWidget *parent = nullptr);
#endif
    ~DisplaySettings() override;

    void setProfile(Profile *p);
private:
#ifndef __APPLE__
    QWidget *buildDevice(QList<VkPhysicalDevice> &&vkDevices);
#endif
    QWidget *buildResolution();

#ifndef __APPLE__
    QComboBox *m_devices;
#endif
    QComboBox *m_resolutions;
    Profile *m_profile;
};

#ifndef __APPLE__
class DisplayDevice : public QObject {
    Q_OBJECT
public:
    DisplayDevice(VkPhysicalDevice handle);

    const char *name() const { return m_props.deviceName; }
private:
    VkPhysicalDevice m_handle;
    VkPhysicalDeviceProperties m_props;
};
#endif
