#pragma once

#include <QList>
#include <QMainWindow>
#ifndef __APPLE__
#include <QVulkanInstance>
#endif

class LaunchSettings;
class QCommandLineOption;
class QCommandLineParser;
class QSocketNotifier;
class QStackedWidget;

class MainWindow final : public QMainWindow {
public:
    MainWindow(const QCommandLineParser &args);
    ~MainWindow() override;

    void restoreGeometry();
protected:
    void closeEvent(QCloseEvent *event) override;
private slots:
    void openSystemFolder();
    void reportIssue();
    void aboutObliteration();
private:
    void vmmError(const QString &msg);
    void waitKernelExit(bool success);
    void stopDebug();
    void killVmm();

    const QCommandLineParser &m_args;
    QStackedWidget *m_main;
    LaunchSettings *m_launch;
    QSocketNotifier *m_debugNoti;
};

namespace Args {
    extern const QCommandLineOption debug;
    extern const QCommandLineOption kernel;
}
