#pragma once

#include "emulator.hpp"

#include <QMainWindow>

class QListView;

class MainWindow final : public QMainWindow {
public:
    MainWindow(emulator_t emulator);
    ~MainWindow();

public:
    void reloadGames();

protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void openGamesFolder();
    void startGame(const QModelIndex &index);
    void requestGamesContextMenu(const QPoint &pos);

private:
    void restoreGeometry();
    bool requireEmulatorStopped();

private:
    emulator_t m_emulator;
    QListView *m_games;
};
