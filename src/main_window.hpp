#pragma once

#include "emulator.hpp"

#include <QMainWindow>

class QListView;

class MainWindow final : public QMainWindow {
public:
    MainWindow(context_t emulator);
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
    context_t m_emulator;
    QListView *m_games;
};
