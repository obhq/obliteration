#pragma once

#include "core.hpp"

#include <QList>
#include <QMainWindow>
#include <QPointer>
#ifndef __APPLE__
#include <QVulkanInstance>
#endif

class GameListModel;
class LaunchSettings;
class LogsViewer;
class ProfileList;
class QStackedWidget;
class Screen;

class MainWindow final : public QMainWindow {
public:
#ifdef __APPLE__
    MainWindow();
#else
    MainWindow(QVulkanInstance *vulkan, QList<VkPhysicalDevice> &&vkDevices);
#endif
    ~MainWindow() override;

    bool loadProfiles();
    bool loadGames();
    void restoreGeometry();
protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void installPkg();
    void openSystemFolder();
    void viewLogs();
    void reportIssue();
    void aboutObliteration();
    void saveProfile(Profile *p);
    void startKernel();
    void updateScreen();
private:
    void log(VmmLog type, const QString &msg);
    bool loadGame(const QString &gameId);
    bool requireEmulatorStopped();

    static bool vmmHandler(const VmmEvent *ev, void *cx);

    QStackedWidget *m_main;
    ProfileList *m_profiles;
    GameListModel *m_games;
    LaunchSettings *m_launch;
    Screen *m_screen;
    QPointer<LogsViewer> m_logs;
    Rust<Vmm> m_kernel; // Destroy first.
};
