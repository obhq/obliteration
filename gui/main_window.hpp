#pragma once

#include "core.hpp"

#include <QList>
#include <QMainWindow>
#ifndef __APPLE__
#include <QVulkanInstance>
#endif

class GameListModel;
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
    bool loadGames();
    void restoreGeometry();
    void startDebug(const QString &addr);
    void startVmm(Rust<DebugClient> &&debug);
protected:
    void closeEvent(QCloseEvent *event) override;
private slots:
    void installPkg();
    void openSystemFolder();
    void reportIssue();
    void aboutObliteration();
    void saveProfile(Profile *p);
    void updateScreen();
private:
    void debuggerConnected();
    void vmmError(const QString &msg);
    void waitKernelExit(bool success);
    void log(VmmLog type, const QString &msg);
    void setupDebugger();
    void dispatchDebug(KernelStop *stop);
    bool loadGame(const QString &gameId);
    bool requireVmmStopped();
    void stopDebug();
    void killVmm();

    static void vmmHandler(const VmmEvent *ev, void *cx);

    const QCommandLineParser &m_args;
    QStackedWidget *m_main;
    ProfileList *m_profiles;
    GameListModel *m_games;
    LaunchSettings *m_launch;
    Screen *m_screen;
    Rust<DebugServer> m_debugServer;
    QSocketNotifier *m_debugNoti;
    Rust<Vmm> m_vmm; // Destroy first.
};

namespace Args {
    extern const QCommandLineOption debug;
    extern const QCommandLineOption kernel;
}
