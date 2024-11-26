#pragma once

#include "core.hpp"

#include <QList>
#include <QMainWindow>
#ifndef __APPLE__
#include <QVulkanInstance>
#endif

class LaunchSettings;
class ProfileList;
class QCommandLineOption;
class QCommandLineParser;
class QSocketNotifier;
class QStackedWidget;
class Screen;

class MainWindow final : public QMainWindow {
public:
#ifdef __APPLE__
    MainWindow(const QCommandLineParser &args);
#else
    MainWindow(
        const QCommandLineParser &args,
        QVulkanInstance *vulkan,
        QList<VkPhysicalDevice> &&vkDevices);
#endif
    ~MainWindow() override;

    bool loadProfiles();
    void restoreGeometry();
protected:
    void closeEvent(QCloseEvent *event) override;
private slots:
    void openSystemFolder();
    void reportIssue();
    void aboutObliteration();
    void saveProfile(Profile *p);
private:
    void vmmError(const QString &msg);
    void waitKernelExit(bool success);
    void stopDebug();
    void killVmm();

    const QCommandLineParser &m_args;
    QStackedWidget *m_main;
    ProfileList *m_profiles;
    LaunchSettings *m_launch;
    Screen *m_screen;
    QSocketNotifier *m_debugNoti;
};

namespace Args {
    extern const QCommandLineOption debug;
    extern const QCommandLineOption kernel;
}
