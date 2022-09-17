#include "main_window.hpp"
#include "game_models.hpp"
#include "settings.hpp"

#include <QAction>
#include <QCloseEvent>
#include <QGuiApplication>
#include <QListView>
#include <QMenuBar>
#include <QMessageBox>
#include <QSettings>

MainWindow::MainWindow(GameListModel *games)
{
    restoreGeometry();

    // Setup File menu.
    auto file = menuBar()->addMenu("&File");
    auto quit = new QAction("&Quit", this);

    connect(quit, &QAction::triggered, this, &MainWindow::close);

    file->addAction(quit);

    // Setup game list.
    m_games = new QListView(this);
    m_games->setViewMode(QListView::IconMode);
    m_games->setModel(games);

    setCentralWidget(m_games);

    // Setup status bar.
    statusBar();
}

MainWindow::~MainWindow()
{
}

void MainWindow::closeEvent(QCloseEvent *event)
{
    // Ask user to confirm.
    QMessageBox confirm(this);

    confirm.setText("Do you want to exit?");
    confirm.setInformativeText("All running games will be terminated.");
    confirm.setStandardButtons(QMessageBox::Cancel | QMessageBox::Yes);
    confirm.setDefaultButton(QMessageBox::Cancel);
    confirm.setIcon(QMessageBox::Warning);

    if (confirm.exec() != QMessageBox::Yes) {
        event->ignore();
        return;
    }

    // Save gometry.
    QSettings settings;

    settings.beginGroup(SettingGroups::mainWindow);
    settings.setValue("size", size());

    if (qGuiApp->platformName() != "wayland") {
        // Wayland does not allow application to position itself.
        settings.setValue("pos", pos());
    }

    QMainWindow::closeEvent(event);
}

void MainWindow::restoreGeometry()
{
    QSettings settings;

    settings.beginGroup(SettingGroups::mainWindow);

    resize(settings.value("size", QSize(800, 800)).toSize());

    if (qGuiApp->platformName() != "wayland") {
        move(settings.value("pos", QPoint(200, 200)).toPoint());
    }
}
