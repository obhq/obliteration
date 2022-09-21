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
    auto gameList = reinterpret_cast<GameListModel *>(m_games->model());

    progress.setLabelText("Loading games...");

    for (auto &dir : games) {
        // Check cancellation.
        if (progress.wasCanceled()) {
            return false;
        }

        // Get full path.
        auto path = joinPath(directory, dir);

        // Add to list.
        gameList->add(new Game(dir, path.c_str()));
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
    auto path = QDir::toNativeSeparators(QFileDialog::getOpenFileName(this, "Install PKG", QString(), "PKG Files (*.pkg)")).toStdString();

    if (path.empty()) {
        return;
    }

    // Prepare a directory to install.
    auto gamesDirectory = readGamesDirectorySetting();
    auto directory = joinPath(gamesDirectory, "installing");

    if (!QDir(directory.c_str()).removeRecursively()) {
        QMessageBox::critical(this, "Error", "Failed to remove previous installation cache.");
        return;
    }

    if (!QDir().mkpath(directory.c_str())) {
        QMessageBox::critical(this, "Error", QString("Cannot create %1").arg(directory.c_str()));
        return;
    }

    // Open a PKG.
    pkg *pkg;
    char *error;

    pkg = pkg_open(m_context, path.c_str(), &error);

    if (!pkg) {
        QMessageBox::critical(this, "Error", QString("Cannot open %1: %2").arg(path.c_str()).arg(error));
        std::free(error);
        return;
    }

    // Read files.
    auto failed = pkg_enum_entries(pkg, [](const pkg_entry *entry, std::size_t, void *ctx) -> void * {
        // Get file name.
        const char *name;

        switch (pkg_entry_id(entry)) {
        case PKG_ENTRY_PARAM_SFO:
            name = "param.sfo";
            break;
        case PKG_ENTRY_PIC1_PNG:
            name = "pic1.png";
            break;
        case PKG_ENTRY_ICON0_PNG:
            name = "icon0.png";
            break;
        default:
            return nullptr;
        }

        // Write file.
        auto path = joinPath(reinterpret_cast<const char *>(ctx), name);
        auto error = pkg_entry_dump(entry, path.c_str());

        if (error) {
            auto message = QString("Failed to write %1 to %2: %3").arg(name).arg(path.c_str()).arg(error);
            std::free(error);
            return new QString(message);
        }

        return nullptr;
    }, const_cast<char *>(directory.c_str()));

    pkg_close(pkg);

    if (failed) {
        auto reason = reinterpret_cast<QString *>(failed);
        QMessageBox::critical(this, "Error", *reason);
        delete reason;
        return;
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
    if (emulator_running(m_context)) {
        QMessageBox::critical(this, "Error", "This functionality is not available while a game is running.");
        return false;
    }

    return true;
}
