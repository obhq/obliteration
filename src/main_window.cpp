#include "main_window.hpp"
#include "emulator.hpp"
#include "game_models.hpp"
#include "game_settings_dialog.hpp"
#include "settings.hpp"

#include <QAction>
#include <QCloseEvent>
#include <QGuiApplication>
#include <QListView>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QSettings>

#include <cstring>

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
    m_games->setContextMenuPolicy(Qt::CustomContextMenu);
    m_games->setModel(games);

    connect(m_games, &QAbstractItemView::doubleClicked, this, &MainWindow::startGame);
    connect(m_games, &QWidget::customContextMenuRequested, this, &MainWindow::requestGamesContextMenu);

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

void MainWindow::startGame(const QModelIndex &index)
{
    // Get target game.
    auto model = reinterpret_cast<GameListModel *>(m_games->model());
    auto game = model->get(index.row()); // Qt already guaranteed the index is valid.

    // Setup config.
    emulator_config conf;

    std::memset(&conf, 0, sizeof(conf));

    emulator_start(&conf);
}

void MainWindow::requestGamesContextMenu(const QPoint &pos)
{
    // Get item index.
    auto index = m_games->indexAt(pos);

    if (!index.isValid()) {
        return;
    }

    auto model = reinterpret_cast<GameListModel *>(m_games->model());
    auto game = model->get(index.row());

    // Setup menu.
    QMenu menu(this);
    QAction settings("&Settings", this);

    menu.addAction(&settings);

    // Show menu.
    auto selected = menu.exec(m_games->viewport()->mapToGlobal(pos));

    if (!selected) {
        return;
    }

    if (selected == &settings) {
        GameSettingsDialog dialog(game, this);
        dialog.exec();
    }
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
