#pragma once

#include <QMainWindow>

class GameListModel;
class QListView;

class MainWindow final : public QMainWindow {
public:
    MainWindow(GameListModel *games);
    ~MainWindow();

private slots:
    void quit();

private:
    QListView *m_games;
};
