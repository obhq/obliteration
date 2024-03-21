#pragma once

#include <QMainWindow>
#include <QModelIndex>
#include <QProcess>

class Debugger;
class LogFormatter;
class QListView;
class SymbolResolver;

class MainWindow final : public QMainWindow {
public:
    MainWindow();
    ~MainWindow();

public:
    bool loadGames();

protected:
    void closeEvent(QCloseEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;

private slots:
    void tabChanged();
    void installPkg();
    void restartGame();
    void openSystemFolder();
    void reportIssue();
    void aboutObliteration();
    void requestGamesContextMenu(const QPoint &pos);
    void startGame(const QModelIndex &index);
    void kernelCrashed();
    void kernelError(QProcess::ProcessError error);
    void kernelOutput();
    void kernelStarted();
    void kernelTerminated(int exitCode, QProcess::ExitStatus exitStatus);

private:
    bool loadGame(const QString &gameId);
    void setLastGame(const QModelIndex &index);
    void killKernel();
    void restoreGeometry();
    bool requireEmulatorStopped();

private:
    QTabWidget *m_tab;
    QListView *m_games;
    QAction *m_restart_game;
    QModelIndex m_last_index;
    LogFormatter *m_log;
    QProcess *m_kernel;
    Debugger *m_debugger;
    SymbolResolver* m_symbol_resolver;
};
