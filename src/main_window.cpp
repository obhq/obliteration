#include "main_window.hpp"
#include "app_data.hpp"
#include "game_models.hpp"
#include "game_settings_dialog.hpp"
#include "pkg.hpp"
#include "progress_dialog.hpp"
#include "settings.hpp"
#include "string.hpp"
#include "util.hpp"

#include <QAction>
#include <QApplication>
#include <QCloseEvent>
#include <QDesktopServices>
#include <QGuiApplication>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QIcon>
#include <QListView>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProgressDialog>
#include <QSettings>
#include <QTabWidget>
#include <QToolBar>
#include <QUrl>

#include <filesystem>

MainWindow::MainWindow(context *context) :
    m_context(context),
    m_tab(nullptr),
    m_games(nullptr),
    m_log(nullptr),
    m_kernel(nullptr)
{
    setWindowTitle("Obliteration");
    restoreGeometry();

    // File menu.
    auto fileMenu = menuBar()->addMenu("&File");
    auto installPkg = new QAction(QIcon(":/resources/archive-arrow-down-outline.svg"), "&Install PKG", this);
    auto quit = new QAction("&Quit", this);

    connect(installPkg, &QAction::triggered, this, &MainWindow::installPkg);
    connect(quit, &QAction::triggered, this, &MainWindow::close);

    fileMenu->addAction(installPkg);
    fileMenu->addSeparator();
    fileMenu->addAction(quit);

    // Help menu.
    auto helpMenu = menuBar()->addMenu("&Help");
    auto reportIssue = new QAction("&Report Issue", this);
    auto aboutQt = new QAction("About &Qt", this);
    auto about = new QAction("&About Obliteration", this);

    connect(reportIssue, &QAction::triggered, this, &MainWindow::reportIssue);
    connect(aboutQt, &QAction::triggered, &QApplication::aboutQt);
    connect(about, &QAction::triggered, this, &MainWindow::aboutObliteration);

    helpMenu->addAction(reportIssue);
    helpMenu->addSeparator();
    helpMenu->addAction(aboutQt);
    helpMenu->addAction(about);

    // File toolbar.
    auto fileBar = addToolBar("&File");

    fileBar->setMovable(false);

    fileBar->addAction(installPkg);

    // Central widget.
    m_tab = new QTabWidget(this);

    setCentralWidget(m_tab);

    // Setup game list.
    m_games = new QListView();
    m_games->setViewMode(QListView::IconMode);
    m_games->setWordWrap(true);
    m_games->setContextMenuPolicy(Qt::CustomContextMenu);
    m_games->setModel(new GameListModel(this));

    connect(m_games, &QAbstractItemView::doubleClicked, this, &MainWindow::startGame);
    connect(m_games, &QWidget::customContextMenuRequested, this, &MainWindow::requestGamesContextMenu);

    m_tab->addTab(m_games, QIcon(":/resources/view-comfy.svg"), "Games");

    // Setup log view.
    m_log = new QPlainTextEdit();
    m_log->setReadOnly(true);
    m_log->setLineWrapMode(QPlainTextEdit::NoWrap);
    m_log->setMaximumBlockCount(10000);

#ifdef _WIN32
    m_log->document()->setDefaultFont(QFont("Courier New", 10));
#else
    m_log->document()->setDefaultFont(QFont("monospace", 10));
#endif

    m_tab->addTab(m_log, QIcon(":/resources/card-text-outline.svg"), "Log");

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
    if (m_kernel) {
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

        killKernel();
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

    // Show dialog to display progress.
    ProgressDialog progress("Install PKG", "Opening PKG...", this);

    // Open a PKG.
    pkg *pkg;
    char *error;

    pkg = pkg_open(m_context, pkgPath.c_str(), &error);

    if (!pkg) {
        QMessageBox::critical(&progress, "Error", QString("Cannot open %1: %2").arg(pkgPath.c_str()).arg(error));
        std::free(error);
        return;
    }

    // Get game ID.
    auto param = pkg_get_param(pkg, &error);

    if (!param) {
        QMessageBox::critical(&progress, "Error", QString("Failed to get param.sfo from %1: %2").arg(pkgPath.c_str()).arg(error));
        std::free(error);
        pkg_close(pkg);
        return;
    }

    auto gameId = fromMalloc(pkg_param_title_id(param));
    auto gameTitle = fromMalloc(pkg_param_title(param));

    pkg_param_close(param);

    // Create game directory.
    auto gamesDirectory = readGamesDirectorySetting();

    if (!QDir(gamesDirectory).mkdir(gameId)) {
        QString msg("Cannot create a directory %1 inside %2.");

        msg += " If you have an unsuccessful installation from the previous attempt you need to remove this directory before install again.";

        QMessageBox::critical(&progress, "Error", msg.arg(gameId).arg(gamesDirectory));
        pkg_close(pkg);
        return;
    }

    auto directory = joinPath(gamesDirectory, gameId);

    // Dump entries.
    Error newError;

    newError = pkg_dump_entries(pkg, directory.c_str());

    if (newError) {
        QMessageBox::critical(&progress, "Error", QString("Failed to extract PKG entries: %1").arg(newError.message()));
        pkg_close(pkg);
        return;
    }

    // Dump PFS.
    progress.setWindowTitle(gameTitle);

    newError = pkg_dump_pfs(pkg, directory.c_str(), [](std::uint64_t written, std::uint64_t total, const char *name, void *ud) {
        auto toProgress = [total](std::uint64_t v) -> int {
            if (total >= 1024UL*1024UL*1024UL*1024UL) { // >= 1TB
                return v / (1024UL*1024UL*1024UL*10UL); // 10GB step.
            } else if (total >= 1024UL*1024UL*1024UL*100UL) { // >= 100GB
                return v / (1024UL*1024UL*1024UL); // 1GB step.
            } else if (total >= 1024UL*1024UL*1024UL*10UL) { // >= 10GB
                return v / (1024UL*1024UL*100UL); // 100MB step.
            } else if (total >= 1024UL*1024UL*1024UL) { // >= 1GB
                return v / (1024UL*1024UL*10UL); // 10MB step.
            } else if (total >= 1024UL*1024UL*100UL) { // >= 100MB
                return v / (1024UL*1024UL);// 1MB step.
            } else {
                return v;
            }
        };

        auto progress = reinterpret_cast<ProgressDialog *>(ud);
        auto max = toProgress(total);
        auto value = toProgress(written);
        auto label = QString("Installing %1...").arg(name);

        if (progress->statusText() != label) {
            progress->setStatusText(label);
            progress->setValue(0);
            progress->setMaximum(max);
        } else {
            progress->setValue(value == max && written != total ? value - 1 : value);
        }
    }, &progress);

    pkg_close(pkg);
    progress.complete();

    if (newError) {
        QMessageBox::critical(this, "Error", QString("Failed to extract pfs_image.dat: %1").arg(newError.message()));
        return;
    }

    // Add to game list.
    auto success = loadGame(gameId);

    if (success) {
        QMessageBox::information(this, "Success", "Installation completed successfully.");
    }
}

void MainWindow::reportIssue()
{
    if (!QDesktopServices::openUrl(QUrl("https://github.com/ultimaweapon/obliteration/issues"))) {
        QMessageBox::critical(this, "Error", "Failed to open https://github.com/ultimaweapon/obliteration/issues with your browser.");
    }
}

void MainWindow::aboutObliteration()
{
    QMessageBox::about(this, "About Obliteration", "Obliteration is a free and open-source software for playing your PlayStation 4 titles on PC.");
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

void MainWindow::startGame(const QModelIndex &index)
{
    if (!requireEmulatorStopped()) {
        return;
    }

    // Get target game.
    auto model = reinterpret_cast<GameListModel *>(m_games->model());
    auto game = model->get(index.row()); // Qt already guaranteed the index is valid.

    // Clear previous log and switch to log view.
    m_log->clear();
    m_tab->setCurrentIndex(1);

    // Get full path to kernel binary.
    QString path;

    if (QFile::exists(".obliteration-development")) {
        auto b = std::filesystem::current_path();

        b /= STR("src");
        b /= STR("target");
#ifdef NDEBUG
        b /= STR("release");
#else
        b /= STR("debug");
#endif

#ifdef _WIN32
        b /= L"obkrnl.exe";
        path = QString::fromStdWString(b.wstring());
#else
        b /= "obkrnl";
        path = QString::fromStdString(b.string());
#endif
    } else {
#ifdef _WIN32
        std::filesystem::path b(QCoreApplication::applicationDirPath().toStdString(), std::filesystem::path::native_format);
        b /= L"bin";
        b /= L"obkrnl.exe";
        path = QString::fromStdWString(b.wstring());
#else
        std::filesystem::path b(QCoreApplication::applicationDirPath().toStdString(), std::filesystem::path::native_format);
        b /= "obkrnl";
        path = QString::fromStdString(b.string());
#endif
    }

    // Setup kernel arguments.
    QStringList args;

    args << "--game" << game->directory();
    args << "--debug-dump" << kernelDebugDump();
    args << "--clear-debug-dump";

    // Prepare kernel launching.
    m_kernel = new QProcess(this);
    m_kernel->setProgram(path);
    m_kernel->setArguments(args);
    m_kernel->setProcessChannelMode(QProcess::MergedChannels);

    connect(m_kernel, &QProcess::errorOccurred, this, &MainWindow::kernelError);
    connect(m_kernel, &QIODevice::readyRead, this, &MainWindow::kernelOutput);
    connect(m_kernel, &QProcess::finished, this, &MainWindow::kernelTerminated);

    // Launch kernel.
    m_kernel->start(QIODeviceBase::ReadOnly | QIODeviceBase::Text);
}

void MainWindow::kernelError(QProcess::ProcessError error)
{
    // Display error.
    QString msg;

    switch (error) {
    case QProcess::FailedToStart:
        msg = QString("Failed to launch %1.").arg(m_kernel->program());
        break;
    case QProcess::Crashed:
        msg = "The kernel crashed.";
        break;
    default:
        msg = "An unknown error occurred on the kernel";
    }

    QMessageBox::critical(this, "Error", msg);

    // Destroy object.
    m_kernel->deleteLater();
    m_kernel = nullptr;
}

void MainWindow::kernelOutput()
{
    while (m_kernel->canReadLine()) {
        auto line = m_kernel->readLine();

        if (line.endsWith('\n')) {
            line.chop(1);
        }

        m_log->appendPlainText(QString::fromUtf8(line));
    }
}

void MainWindow::kernelTerminated(int, QProcess::ExitStatus)
{
    kernelOutput();

    QMessageBox::critical(this, "Error", "The emulator kernel has been stopped unexpectedly. Please take a look on the log and report the issue.");

    m_kernel->deleteLater();
    m_kernel = nullptr;
}

bool MainWindow::loadGame(const QString &gameId)
{
    auto gamesDirectory = readGamesDirectorySetting();
    auto gamePath = joinPath(gamesDirectory, gameId);
    auto gameList = reinterpret_cast<GameListModel *>(m_games->model());

    // Read game title from param.sfo.
    auto paramPath = joinPath(gamePath.c_str(), PKG_ENTRY_PARAM_SFO);
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

void MainWindow::killKernel()
{
    // We need to disconnect all slots first otherwise the application will be freezing.
    disconnect(m_kernel, nullptr, nullptr, nullptr);

    m_kernel->kill();
    m_kernel->waitForFinished(-1);

    delete m_kernel;
    m_kernel = nullptr;
}

void MainWindow::restoreGeometry()
{
    QSettings settings;

    settings.beginGroup(SettingGroups::mainWindow);

    resize(settings.value("size", QSize(1000, 500)).toSize());

    if (qGuiApp->platformName() != "wayland") {
        move(settings.value("pos", QPoint(200, 200)).toPoint());
    }
}

bool MainWindow::requireEmulatorStopped()
{
    if (m_kernel) {
        QMessageBox::critical(this, "Error", "This functionality is not available while a game is running.");
        return false;
    }

    return true;
}
