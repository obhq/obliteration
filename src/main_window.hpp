#pragma once

#include "context.hpp"

#include <QMainWindow>

class QListView;

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
    void restoreGeometry();
    bool requireEmulatorStopped();

private:
    context *m_context;
    QListView *m_games;
};
