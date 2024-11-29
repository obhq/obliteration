#pragma once

#include <QList>
#ifndef __APPLE__
#include <QVulkanInstance>
#endif
#include <QWidget>

class CpuSettings;
#ifndef __APPLE__
class DisplayDevice;
#endif
class DisplaySettings;
class GameListModel;
class ProfileList;
class QComboBox;
class QLayout;
class QTableView;

class LaunchSettings final : public QWidget {
    Q_OBJECT
public:
#ifdef __APPLE__
    LaunchSettings(QWidget *parent = nullptr);
#else
    LaunchSettings(
        QList<VkPhysicalDevice> &&vkDevices,
        QWidget *parent = nullptr);
#endif
    ~LaunchSettings() override;

#ifndef __APPLE__
    DisplayDevice *currentDisplayDevice() const;
#endif
signals:
    void startClicked(const QString &debugAddr);
private:
#ifdef __APPLE__
    QWidget *buildSettings();
#else
    QWidget *buildSettings(QList<VkPhysicalDevice> &&vkDevices);
#endif
    QLayout *buildActions();

    DisplaySettings *m_display;
    CpuSettings *m_cpu;
    QComboBox *m_profiles;
};
