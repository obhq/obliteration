#pragma once

#ifndef __APPLE__
#include <QVulkanInstance>
#endif
#include <QWidget>

#ifndef __APPLE__
class DisplayDevice;
#endif
class QComboBox;

class DisplaySettings final : public QWidget {
public:
#ifdef __APPLE__
    DisplaySettings(QWidget *parent = nullptr);
#else
    DisplaySettings(QList<VkPhysicalDevice> &&vkDevices, QWidget *parent = nullptr);
#endif
    ~DisplaySettings() override;

#ifndef __APPLE__
    DisplayDevice *currentDevice() const;
#endif
private:
#ifndef __APPLE__
    QWidget *buildDevice(QList<VkPhysicalDevice> &&vkDevices);
#endif

#ifndef __APPLE__
    QComboBox *m_devices;
#endif
    QComboBox *m_resolutions;
};

#ifndef __APPLE__
class DisplayDevice : public QObject {
    Q_OBJECT
public:
    DisplayDevice(VkPhysicalDevice handle);

    const char *name() const { return m_props.deviceName; }
    VkPhysicalDevice handle() const { return m_handle; }
private:
    VkPhysicalDevice m_handle;
    VkPhysicalDeviceProperties m_props;
};
#endif
