#pragma once

#include <QList>
#include <QMainWindow>

class QStackedWidget;

class MainWindow final : public QMainWindow {
public:
    MainWindow();
    ~MainWindow() override;
private slots:
    void aboutObliteration();
private:
    QStackedWidget *m_main;
};
