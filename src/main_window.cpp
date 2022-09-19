#include "main_window.hpp"
#include "game_models.hpp"
#include "game_settings_dialog.hpp"
#include "settings.hpp"

#include <QAction>
#include <QCloseEvent>
#include <QGuiApplication>
#include <QDir>
#include <QFileDialog>
#include <QIcon>
#include <QListView>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QProgressDialog>
#include <QTabWidget>
#include <QToolBar>
#include <QSettings>

#include <cstring>
#include <filesystem>

MainWindow::MainWindow(emulator_t emulator) :
    m_emulator(emulator),
    m_games(nullptr)
{
    setWindowTitle("Obliteration");
    restoreGeometry();

    // Setup File menu.
    auto fileMenu = menuBar()->addMenu("&File");
    auto openGames = new QAction(QIcon(":/resources/folder-open-outline.svg"), "&Open Games Folder", this);
    auto quit = new QAction("&Quit", this);

    connect(openGames, &QAction::triggered, this, &MainWindow::openGamesFolder);
    connect(quit, &QAction::triggered, this, &MainWindow::close);

    fileMenu->addAction(openGames);
    fileMenu->addSeparator();
    fileMenu->addAction(quit);

    // Setup File toolbar.
    auto fileBar = addToolBar("&File");

    fileBar->setMovable(false);

    fileBar->addAction(openGames);

    // Setup game list.
    m_games = new QListView();
    m_games->setViewMode(QListView::IconMode);
    m_games->setContextMenuPolicy(Qt::CustomContextMenu);
    m_games->setModel(new GameListModel(this));

    connect(m_games, &QAbstractItemView::doubleClicked, this, &MainWindow::startGame);
    connect(m_games, &QWidget::customContextMenuRequested, this, &MainWindow::requestGamesContextMenu);

    // Setup central widget.
    auto tab = new QTabWidget(this);

    tab->addTab(m_games, "Games");

    setCentralWidget(tab);

    // Setup status bar.
    statusBar();
}

MainWindow::~MainWindow()
{
}

void MainWindow::reloadGames()
{
    if (!requireEmulatorStopped()) {
        return;
    }

    // Get game counts.
    auto directory = readGamesDirectorySetting();

    if (directory.isNull()) {
        return;
    }

    auto pkgs = QDir(directory, "*.pkg").entryList();

    // Remove current games.
    auto games = reinterpret_cast<GameListModel *>(m_games->model());

    games->clear();

    // Setup loading progress.
    QProgressDialog progress(this);
    int step = -1;

    progress.setMaximum(pkgs.size());
    progress.setCancelButtonText("Cancel");
    progress.setWindowModality(Qt::WindowModal);
    progress.setValue(++step);

    // Load games
    progress.setLabelText("Loading games...");

    for (auto &file : pkgs) {
        // Check cancellation.
        if (progress.wasCanceled()) {
            QCoreApplication::exit();
            return;
        }

        // Get full path.
        std::string path;

        try {
            std::filesystem::path b(directory.toStdString(), std::filesystem::path::native_format);

            b /= file.toStdString();

            path = b.u8string();
        } catch (...) {
            QMessageBox::critical(this, "Fatal Error", QString("An unexpected error occurred while reading %1.").arg(file));
            QCoreApplication::exit();
            return;
        }

        // Read PKG meta data.
        emulator_pkg_t pkg;
        char *error;

        pkg = emulator_pkg_open(m_emulator, path.c_str(), &error);

        if (!pkg) {
            QMessageBox::critical(this, "Fatal Error", QString("Failed to open %1: %2").arg(path.c_str()).arg(error));
            std::free(error);
            QCoreApplication::exit();
            return;
        }

        emulator_pkg_close(pkg);

        // Add to list.
        games->add(new Game(path.c_str()));
        progress.setValue(++step);
    }
}

void MainWindow::closeEvent(QCloseEvent *event)
{
    // Ask user to confirm.
    if (emulator_running(m_emulator)) {
        QMessageBox confirm(this);

        confirm.setText("Do you want to exit?");
        confirm.setInformativeText("The running game will be terminated.");
        confirm.setStandardButtons(QMessageBox::Cancel | QMessageBox::Yes);
        confirm.setDefaultButton(QMessageBox::Cancel);
        confirm.setIcon(QMessageBox::Warning);

        if (confirm.exec() != QMessageBox::Yes) {
            event->ignore();
            return;
        }
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

void MainWindow::openGamesFolder()
{
    if (!requireEmulatorStopped()) {
        return;
    }

    // Browse folder.
    auto path = QFileDialog::getExistingDirectory(this, "Location for PKG files");

    if (path.isEmpty()) {
        return;
    }

    path = QDir::toNativeSeparators(path);

    // Write setting and reload game list.
    writeGamesDirectorySetting(path);
    reloadGames();
}

void MainWindow::startGame(const QModelIndex &index)
{
    // Get target game.
    auto model = reinterpret_cast<GameListModel *>(m_games->model());
    auto game = model->get(index.row()); // Qt already guaranteed the index is valid.

    // Setup config.
    emulator_config conf;

    std::memset(&conf, 0, sizeof(conf));

    emulator_start(m_emulator, &conf);
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

    resize(settings.value("size", QSize(1000.0 * devicePixelRatioF(), 600.0 * devicePixelRatioF())).toSize());

    if (qGuiApp->platformName() != "wayland") {
        move(settings.value("pos", QPoint(200.0 * devicePixelRatioF(), 200.0 * devicePixelRatioF())).toPoint());
    }
}

bool MainWindow::requireEmulatorStopped()
{
    if (emulator_running(m_emulator)) {
        QMessageBox::critical(this, "Error", "This functionality is not available while a game is running.");
        return false;
    }

    return true;
}
