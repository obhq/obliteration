#pragma once

#include <QMainWindow>

class QListView;

class MainWindow final : public QMainWindow {
public:
    MainWindow(void *emulator);
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
    void *m_emulator;
    QListView *m_games;
};
