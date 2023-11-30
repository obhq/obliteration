#pragma once

#include <QMainWindow>
#include <QProcess>

class LogFormatter;
class QListView;

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
    void openSystemFolder();
    void reportIssue();
    void aboutObliteration();
    void requestGamesContextMenu(const QPoint &pos);
    void startGame(const QModelIndex &index);
    void kernelError(QProcess::ProcessError error);
    void kernelOutput();
    void kernelTerminated(int exitCode, QProcess::ExitStatus exitStatus);

private:
    bool loadGame(const QString &gameId, bool patchLoad);
    void killKernel();
    void restoreGeometry();
    bool requireEmulatorStopped();

private:
    QTabWidget *m_tab;
    QListView *m_games;
    LogFormatter *m_log;
    QProcess *m_kernel;
};
