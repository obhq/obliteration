#pragma once

#include <QMainWindow>
#include <QProcess>

class LogFormatter;
class QListView;
class QPlainTextEdit;

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
    void reportIssue();
    void aboutObliteration();
    void requestGamesContextMenu(const QPoint &pos);
    void startGame(const QModelIndex &index);
    void kernelError(QProcess::ProcessError error);
    void kernelOutput();
    void kernelTerminated(int exitCode, QProcess::ExitStatus exitStatus);

private:
    bool loadGame(const QString &gameId);
    void killKernel();
    void restoreGeometry();
    bool requireEmulatorStopped();

private:
    QTabWidget *m_tab;
    QListView *m_games;
    QPlainTextEdit *m_log;
    LogFormatter *m_formatter;
    QProcess *m_kernel;
};
