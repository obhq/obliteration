#pragma once

#include "core.h"

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
    LaunchSettings(ProfileList *profiles, QWidget *parent = nullptr);
#else
    LaunchSettings(
        ProfileList *profiles,
        QList<VkPhysicalDevice> &&vkDevices,
        QWidget *parent = nullptr);
#endif
    ~LaunchSettings() override;

    Profile *currentProfile() const;
#ifndef __APPLE__
    DisplayDevice *currentDisplayDevice() const;
#endif
signals:
    void saveClicked(Profile *p);
    void startClicked(const QString &debugAddr);
private:
#ifdef __APPLE__
    QWidget *buildSettings();
#else
    QWidget *buildSettings(QList<VkPhysicalDevice> &&vkDevices);
#endif
    QLayout *buildActions(ProfileList *profiles);

    void profileChanged(int index);

    DisplaySettings *m_display;
    CpuSettings *m_cpu;
    QTableView *m_games;
    QComboBox *m_profiles;
};
