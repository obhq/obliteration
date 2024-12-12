#pragma once

#include <QList>
#include <QMainWindow>

class MainWindow final : public QMainWindow {
public:
    MainWindow();
    ~MainWindow() override;
private slots:
    void aboutObliteration();
};
