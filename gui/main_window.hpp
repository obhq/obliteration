#pragma once

#include "core.hpp"

#include <QAbstractSocket>
#include <QList>
#include <QMainWindow>
#include <QPointer>
#ifndef __APPLE__
#include <QVulkanInstance>
#endif

#include <optional>

class GameListModel;
class LaunchSettings;
class LogsViewer;
class ProfileList;
class QCommandLineOption;
class QCommandLineParser;
class QStackedWidget;
class QTcpSocket;
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
    void startVmm();
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
    void waitKernelExit(bool success);
    void log(VmmLog type, const QString &msg);
    void breakpoint(KernelStop *stop);
    std::optional<QAbstractSocket::SocketError> sendDebug(const uint8_t *data, size_t len);
    bool loadGame(const QString &gameId);
    bool requireVmmStopped();
    void killVmm();

    static void vmmHandler(const VmmEvent *ev, void *cx);
    static bool sendDebug(void *cx, const uint8_t *data, size_t len, int *err);

    const QCommandLineParser &m_args;
    QStackedWidget *m_main;
    ProfileList *m_profiles;
    GameListModel *m_games;
    LaunchSettings *m_launch;
    Screen *m_screen;
    QPointer<LogsViewer> m_logs;
    QTcpSocket *m_debug;
    Rust<Vmm> m_vmm; // Destroy first.
};

namespace Args {
    extern const QCommandLineOption debug;
}
