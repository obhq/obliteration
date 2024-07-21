#pragma once

#include "core.hpp"

#include <QMainWindow>

class LaunchSettings;
class LogFormatter;
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
    LogFormatter *m_log;
    RustPtr<Vmm> m_kernel;
};
