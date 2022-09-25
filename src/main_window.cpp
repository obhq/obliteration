#include "main_window.hpp"
#include "emulator.hpp"
#include "game_models.hpp"
#include "game_settings_dialog.hpp"
#include "pkg.hpp"
#include "settings.hpp"
#include "util.hpp"

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

MainWindow::MainWindow(context *context) :
    m_context(context),
    m_games(nullptr)
{
    setWindowTitle("Obliteration");
    restoreGeometry();

    // Setup File menu.
    auto fileMenu = menuBar()->addMenu("&File");
    auto installPkg = new QAction(QIcon(":/resources/archive-arrow-down-outline.svg"), "&Install PKG", this);
    auto quit = new QAction("&Quit", this);

    connect(installPkg, &QAction::triggered, this, &MainWindow::installPkg);
    connect(quit, &QAction::triggered, this, &MainWindow::close);

    fileMenu->addAction(installPkg);
    fileMenu->addSeparator();
    fileMenu->addAction(quit);

    // Setup File toolbar.
    auto fileBar = addToolBar("&File");

    fileBar->setMovable(false);

    fileBar->addAction(installPkg);

    // Setup game list.
    m_games = new QListView();
    m_games->setViewMode(QListView::IconMode);
    m_games->setWordWrap(true);
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

bool MainWindow::loadGames()
{
    // Get game counts.
    auto directory = readGamesDirectorySetting();
    auto games = QDir(directory).entryList(QDir::Dirs | QDir::NoDotAndDotDot);

    // Setup loading progress.
    QProgressDialog progress(this);
    int step = -1;

    progress.setMaximum(games.size());
    progress.setCancelButtonText("Cancel");
    progress.setWindowModality(Qt::WindowModal);
    progress.setValue(++step);

    // Load games
    progress.setLabelText("Loading games...");

    for (auto &gameId : games) {
        if (progress.wasCanceled() || !loadGame(gameId)) {
            return false;
        }

        progress.setValue(++step);
    }

    return true;
}

void MainWindow::closeEvent(QCloseEvent *event)
{
    // Ask user to confirm.
    if (emulator_running(m_context)) {
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

void MainWindow::installPkg()
{
    // Browse a PKG.
    auto pkgPath = QDir::toNativeSeparators(QFileDialog::getOpenFileName(this, "Install PKG", QString(), "PKG Files (*.pkg)")).toStdString();

    if (pkgPath.empty()) {
        return;
    }

    // Open a PKG.
    pkg *pkg;
    char *error;

    pkg = pkg_open(m_context, pkgPath.c_str(), &error);

    if (!pkg) {
        QMessageBox::critical(this, "Error", QString("Cannot open %1: %2").arg(pkgPath.c_str()).arg(error));
        std::free(error);
        return;
    }

    // Get game ID.
    auto param = pkg_get_param(pkg, &error);

    if (!param) {
        QMessageBox::critical(this, "Error", QString("Failed to get param.sfo from %1: %2").arg(pkgPath.c_str()).arg(error));
        std::free(error);
        pkg_close(pkg);
        return;
    }

    auto gameId = fromMalloc(pkg_param_title_id(param));

    pkg_param_close(param);

    // Create game directory.
    auto gamesDirectory = readGamesDirectorySetting();

    if (!QDir(gamesDirectory).mkdir(gameId)) {
        QString error("Cannot create a directory %1 inside %2.");

        error += " If you have an unsuccessful installation from the previous attempt you need to remove this directory before install again.";

        QMessageBox::critical(this, "Error", error.arg(gameId).arg(gamesDirectory));
        pkg_close(pkg);
        return;
    }

    auto directory = joinPath(gamesDirectory, gameId);

    // Dump param.sfo.
    error = pkg_dump_entry(pkg, PKG_ENTRY_PARAM_SFO, joinPath(directory.c_str(), "param.sfo").c_str());

    if (error) {
        QMessageBox::critical(this, "Error", QString("Failed to install param.sfo to %1: %2").arg(directory.c_str(), error));
        std::free(error);
        pkg_close(pkg);
        return;
    }

    // Dump pic1.png.
    error = pkg_dump_entry(pkg, PKG_ENTRY_PIC1_PNG, joinPath(directory.c_str(), "pic1.png").c_str());

    if (error) {
        QMessageBox::critical(this, "Error", QString("Failed to install pic1.png to %1: %2").arg(directory.c_str(), error));
        std::free(error);
        pkg_close(pkg);
        return;
    }

    // Dump icon0.png.
    error = pkg_dump_entry(pkg, PKG_ENTRY_ICON0_PNG, joinPath(directory.c_str(), "icon0.png").c_str());

    if (error) {
        QMessageBox::critical(this, "Error", QString("Failed to install pic1.png to %1: %2").arg(directory.c_str(), error));
        std::free(error);
        pkg_close(pkg);
        return;
    }

    // Dump PFS.
    QProgressDialog progress(this);

    progress.setLabelText("Extracting PFS...");
    progress.setCancelButton(nullptr);
    progress.setValue(0);
    progress.setWindowModality(Qt::WindowModal);

    error = pkg_dump_pfs(pkg, directory.c_str(), [](std::size_t written, std::size_t size, void *ud) {
        auto progress = reinterpret_cast<QProgressDialog *>(ud);
        auto toProgress = (size < (1024 * 1024 * 1024)) ? [](std::size_t v) { return static_cast<int>(v); } : [](std::size_t v) { return static_cast<int>(v / (1024 * 1024 * 1024)); };

        if (!progress->value()) {
            progress->setMaximum(toProgress(size));
        }

        progress->setValue(toProgress(written));
    }, &progress);

    pkg_close(pkg);

    if (error) {
        QMessageBox::critical(&progress, "Error", QString("Failed to extract pfs_image.dat: %1").arg(error));
        std::free(error);
        return;
    }

    // Add to game list.
    auto success = loadGame(gameId);

    if (success) {
        QMessageBox::information(this, "Success", "Installation completed successfully.");
    }
}

void MainWindow::startGame(const QModelIndex &index)
{
    // Get target game.
    auto model = reinterpret_cast<GameListModel *>(m_games->model());
    auto game = model->get(index.row()); // Qt already guaranteed the index is valid.

    // Setup config.
    emulator_config conf;

    std::memset(&conf, 0, sizeof(conf));

    emulator_start(m_context, &conf);
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

bool MainWindow::loadGame(const QString &gameId)
{
    auto gamesDirectory = readGamesDirectorySetting();
    auto gamePath = joinPath(gamesDirectory, gameId);
    auto gameList = reinterpret_cast<GameListModel *>(m_games->model());

    // Read game title from param.sfo.
    auto paramPath = joinPath(gamePath.c_str(), "param.sfo");
    pkg_param *param;
    char *error;

    param = pkg_param_open(paramPath.c_str(), &error);

    if (!param) {
        QMessageBox::critical(this, "Error", QString("Cannot open %1: %2").arg(paramPath.c_str()).arg(error));
        std::free(error);
        return false;
    }

    auto name = fromMalloc(pkg_param_title(param));

    pkg_param_close(param);

    // Add to list.
    gameList->add(new Game(name, gamePath.c_str()));

    return true;
}

void MainWindow::restoreGeometry()
{
    QSettings settings;

    settings.beginGroup(SettingGroups::mainWindow);

    resize(settings.value("size", QSize(1000, 600)).toSize());

    if (qGuiApp->platformName() != "wayland") {
        move(settings.value("pos", QPoint(200, 200)).toPoint());
    }
}

bool MainWindow::requireEmulatorStopped()
{
    if (emulator_running(m_context)) {
        QMessageBox::critical(this, "Error", "This functionality is not available while a game is running.");
        return false;
    }

    return true;
}
