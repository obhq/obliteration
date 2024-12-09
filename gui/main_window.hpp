#pragma once

#include <QList>
#include <QMainWindow>

class QStackedWidget;

class MainWindow final : public QMainWindow {
public:
    MainWindow();
    ~MainWindow() override;
private slots:
    void reportIssue();
    void aboutObliteration();
private:
    void waitKernelExit(bool success);

    QStackedWidget *m_main;
};
