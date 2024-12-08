#pragma once

#include <QList>
#include <QMainWindow>

class QCommandLineOption;
class QCommandLineParser;
class QStackedWidget;

class MainWindow final : public QMainWindow {
public:
    MainWindow(const QCommandLineParser &args);
    ~MainWindow() override;
private slots:
    void reportIssue();
    void aboutObliteration();
private:
    void vmmError(const QString &msg);
    void waitKernelExit(bool success);
    void stopDebug();

    const QCommandLineParser &m_args;
    QStackedWidget *m_main;
};

namespace Args {
    extern const QCommandLineOption debug;
    extern const QCommandLineOption kernel;
}
