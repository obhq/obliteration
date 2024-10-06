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
class QCommandLineOption;
class QCommandLineParser;
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
    bool loadGames();
    void restoreGeometry();
    void startVmm(const QString &debugAddr);
protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void installPkg();
    void openSystemFolder();
    void viewLogs();
    void reportIssue();
    void aboutObliteration();
    void saveProfile(Profile *p);
    void updateScreen();
private:
    void vmmError(const QString &msg);
    void waitingDebugger(const QString &addr);
    void debuggerDisconnected();
    void waitKernelExit(bool success);
    void log(VmmLog type, const QString &msg);
    bool loadGame(const QString &gameId);
    bool requireVmmStopped();

    static void vmmHandler(const VmmEvent *ev, void *cx);

    const QCommandLineParser &m_args;
    QStackedWidget *m_main;
    ProfileList *m_profiles;
    GameListModel *m_games;
    LaunchSettings *m_launch;
    Screen *m_screen;
    QPointer<LogsViewer> m_logs;
    Rust<Vmm> m_vmm; // Destroy first.
};

namespace Args {
    extern const QCommandLineOption debug;
}
