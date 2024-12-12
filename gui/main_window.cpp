#include "main_window.hpp"

#include <QAction>
#include <QApplication>
#include <QCloseEvent>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QDesktopServices>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QIcon>
#include <QMenuBar>
#include <QMessageBox>
#include <QProgressDialog>
#include <QResizeEvent>
#include <QScrollBar>
#include <QSettings>
#include <QSocketNotifier>
#include <QStackedWidget>
#include <QToolBar>
#include <QUrl>

#ifndef _WIN32
#include <fcntl.h>
#endif

MainWindow::MainWindow()
{
    // File menu.
    auto fileMenu = menuBar()->addMenu("&File");
    auto openSystemFolder = new QAction("Open System &Folder", this);

    fileMenu->addAction(openSystemFolder);

    // Help menu.
    auto helpMenu = menuBar()->addMenu("&Help");
    auto about = new QAction("&About Obliteration", this);

    connect(about, &QAction::triggered, this, &MainWindow::aboutObliteration);

    helpMenu->addAction(about);
}

MainWindow::~MainWindow()
{
}

void MainWindow::aboutObliteration()
{
    QMessageBox::about(
        this,
        "About Obliteration",
        "Obliteration is a free and open-source PlayStation 4 kernel. It will allows you to run "
        "the PlayStation 4 system software that you have dumped from your PlayStation 4 on your "
        "PC. This will allows you to play your games forever even if your PlayStation 4 stopped "
        "working in the future.");
}
