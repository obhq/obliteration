#pragma once

#include <QMainWindow>

class GameListModel;
class QListView;

class MainWindow final : public QMainWindow {
public:
    MainWindow(GameListModel *games);
    ~MainWindow();

protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void startGame(const QModelIndex &index);
    void requestGamesContextMenu(const QPoint &pos);

private:
    void restoreGeometry();

private:
    QListView *m_games;
};
