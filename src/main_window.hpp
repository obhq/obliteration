#pragma once

#include "core.hpp"

#include <QMainWindow>
#include <QPointer>

class LaunchSettings;
class LogsViewer;
class QStackedWidget;
class QTableView;

class MainWindow final : public QMainWindow {
public:
    MainWindow();
    ~MainWindow();

public:
    bool loadGames();

protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void installPkg();
    void openSystemFolder();
    void viewLogs();
    void reportIssue();
    void aboutObliteration();
    void requestGamesContextMenu(const QPoint &pos);
    void startKernel();

private:
    bool loadGame(const QString &gameId);
    void restoreGeometry();
    bool requireEmulatorStopped();

private:
    QTabWidget *m_tab;
    QStackedWidget *m_screen;
    LaunchSettings *m_launch;
    QTableView *m_games;
    QPointer<LogsViewer> m_logs;
    RustPtr<Vmm> m_kernel;
};
