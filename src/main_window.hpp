#pragma once

#include "core.hpp"

#include <QMainWindow>
#include <QPointer>

class GameListModel;
class LaunchSettings;
class LogsViewer;
class QStackedWidget;
#ifndef __APPLE__
class QVulkanInstance;
#endif
class Screen;

class MainWindow final : public QMainWindow {
public:
#ifdef __APPLE__
    MainWindow();
#else
    MainWindow(QVulkanInstance *vulkan);
#endif
    ~MainWindow();

    bool loadGames();
protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void installPkg();
    void openSystemFolder();
    void viewLogs();
    void reportIssue();
    void aboutObliteration();
    void startKernel();
    void updateScreen();

private:
    bool loadGame(const QString &gameId);
    void restoreGeometry();
    bool requireEmulatorStopped();

    QStackedWidget *m_main;
    GameListModel *m_games;
    LaunchSettings *m_launch;
    Screen *m_screen;
    QPointer<LogsViewer> m_logs;
    Rust<Vmm> m_kernel;
};
