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
        if (progress.wasCanceled() || !loadGame(&progress, gameId)) {
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

    // Prepare a temporary directory to extract PKG entries. We cannot use a standard temporary directory here becuase
    // on Linux it will fail when we try to move it to a games directory.
    auto gamesDirectory = readGamesDirectorySetting();
    auto tempInstallPath = joinPath(gamesDirectory, "installing");

    if (!QDir(tempInstallPath.c_str()).removeRecursively()) {
        QMessageBox::critical(this, "Error", "Failed to remove previous installation cache.");
        return;
    }

    if (!QDir().mkpath(tempInstallPath.c_str())) {
        QMessageBox::critical(this, "Error", QString("Cannot create %1").arg(tempInstallPath.c_str()));
        return;
    }

    // Setup loading progress.
    QProgressDialog progress(this);
    int step = -1;

    progress.setMaximum(5);
    progress.setCancelButtonText("Cancel");
    progress.setWindowModality(Qt::WindowModal);
    progress.setValue(++step);

    // Open a PKG.
    pkg *pkg;
    char *error;

    progress.setLabelText(QString("Opening %1...").arg(pkgPath.c_str()));

    pkg = pkg_open(m_context, pkgPath.c_str(), &error);

    if (!pkg) {
        QMessageBox::critical(&progress, "Error", QString("Cannot open %1: %2").arg(pkgPath.c_str()).arg(error));
        std::free(error);
        return;
    }

    progress.setValue(++step);

    // Dump entries.
    progress.setLabelText("Extracting PKG entries...");

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
    }, const_cast<char *>(tempInstallPath.c_str()));

    pkg_close(pkg);

    if (failed) {
        auto reason = reinterpret_cast<QString *>(failed);
        QMessageBox::critical(&progress, "Error", *reason);
        delete reason;
        return;
    }

    progress.setValue(++step);

    // Get game ID from param.sfo.
    progress.setLabelText("Getting game identifier...");

    auto paramPath = joinPath(tempInstallPath.c_str(), "param.sfo");
    auto param = pkg_param_open(paramPath.c_str(), &error);

    if (!param) {
        QMessageBox::critical(&progress, "Error", QString("Cannot open %1: %2").arg(paramPath.c_str(), error));
        std::free(error);
        QDir(tempInstallPath.c_str()).removeRecursively();
        return;
    }

    auto gameId = fromMalloc(pkg_param_title_id(param));

    pkg_param_close(param);
    progress.setValue(++step);

    // Rename directory to game ID.
    auto installPath = joinPath(gamesDirectory, gameId);

    progress.setLabelText("Extracting PFS...");

    if (!QDir().rename(tempInstallPath.c_str(), installPath.c_str())) {
        QMessageBox::critical(&progress, "Error", QString("Failed to rename %1 to %2.").arg(tempInstallPath.c_str(), installPath.c_str()));
        return;
    }

    progress.setValue(++step);

    // Add to game list.
    progress.setLabelText("Adding to game list...");

    if (!loadGame(&progress, gameId)) {
        QDir(installPath.c_str()).removeRecursively();
    }

    progress.setValue(++step);

    QMessageBox::information(this, "Success", "Installation completed successfully.");
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

bool MainWindow::loadGame(QWidget *progress, const QString &gameId)
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
        QMessageBox::critical(progress, "Error", QString("Cannot open %1: %2").arg(paramPath.c_str()).arg(error));
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
