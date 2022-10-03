#pragma once

#include "context.hpp"
#include "kernel.hpp"

#include <QMainWindow>

class QListView;
class QPlainTextEdit;

class MainWindow final : public QMainWindow {
public:
    MainWindow(context *context);
    ~MainWindow();

public:
    bool loadGames();

protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void installPkg();
    void startGame(const QModelIndex &index);
    void requestGamesContextMenu(const QPoint &pos);

private:
    bool loadGame(const QString &gameId);
    void appendLog(int pid, int err, const char *msg);
    void restoreGeometry();
    bool requireEmulatorStopped();

private:
    context *m_context;
    QTabWidget *m_tab;
    QListView *m_games;
    QPlainTextEdit *m_log;
    kernel *m_kernel;
};
