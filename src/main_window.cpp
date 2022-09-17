#include "main_window.hpp"
#include "game_models.hpp"

#include <QAction>
#include <QCoreApplication>
#include <QListView>
#include <QMenuBar>
#include <QMessageBox>

MainWindow::MainWindow(GameListModel *games)
{
    // Setup File menu.
    auto file = menuBar()->addMenu("&File");
    auto quit = new QAction("&Quit", this);

    connect(quit, &QAction::triggered, this, &MainWindow::quit);

    file->addAction(quit);

    // Setup game list.
    m_games = new QListView(this);
    m_games->setViewMode(QListView::IconMode);
    m_games->setModel(games);

    setCentralWidget(m_games);
}

MainWindow::~MainWindow()
{
}

void MainWindow::quit()
{
    // Ask user to confirm.
    QMessageBox confirm(this);

    confirm.setText("Do you want to exit?");
    confirm.setInformativeText("All running games will be terminated.");
    confirm.setStandardButtons(QMessageBox::Cancel | QMessageBox::Yes);
    confirm.setDefaultButton(QMessageBox::Cancel);
    confirm.setIcon(QMessageBox::Warning);

    if (confirm.exec() != QMessageBox::Yes) {
        return;
    }

    // Exit Qt.
    QCoreApplication::instance()->exit();
}
