#pragma once

#include <QListWidget>
#include <QMainWindow>

class MainWindow final : public QMainWindow {
public:
    MainWindow();
    ~MainWindow();

private slots:
    void quit();

private:
    QListWidget *m_games;
};
