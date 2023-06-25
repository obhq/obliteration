#include "main_window.hpp"
#include "app_data.hpp"
#include "game_models.hpp"
#include "game_settings_dialog.hpp"
#include "log_formatter.hpp"
#include "path.hpp"
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
#include <QStyleHints>
#include <QTabWidget>
#include <QToolBar>
#include <QUrl>

#include <filesystem>

MainWindow::MainWindow() :
    m_tab(nullptr),
    m_games(nullptr),
    m_log(nullptr),
    m_kernel(nullptr)
{
    setWindowTitle("Obliteration");
    restoreGeometry();

    // Determine current theme.
    QString svgPath;

    if (QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark) {
        svgPath = ":/resources/darkmode/";
    } else {
        svgPath = ":/resources/lightmode/";
    }

    // File menu.
    auto fileMenu = menuBar()->addMenu("&File");
    auto installPkg = new QAction(QIcon(svgPath + "archive-arrow-down-outline.svg"), "&Install PKG", this);
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

#ifndef __APPLE__
    // File toolbar.
    auto fileBar = addToolBar("&File");

    fileBar->setMovable(false);
    fileBar->addAction(installPkg);
#endif

    // Central widget.
    m_tab = new QTabWidget(this);
    m_tab->setDocumentMode(true);

#ifdef __APPLE__
    m_tab->tabBar()->setExpanding(true);
#endif

    setCentralWidget(m_tab);

    // Setup game list.
    m_games = new QListView();
    m_games->setViewMode(QListView::IconMode);
    m_games->setWordWrap(true);
    m_games->setContextMenuPolicy(Qt::CustomContextMenu);
    m_games->setModel(new GameListModel(this));

    connect(m_games, &QAbstractItemView::doubleClicked, this, &MainWindow::startGame);
    connect(m_games, &QWidget::customContextMenuRequested, this, &MainWindow::requestGamesContextMenu);

    m_tab->addTab(m_games, QIcon(svgPath + "view-comfy.svg"), "Games");

    // Setup log view.
    auto log = new QPlainTextEdit();

    log->setReadOnly(true);
    log->setLineWrapMode(QPlainTextEdit::NoWrap);
    log->setMaximumBlockCount(10000);

#ifdef _WIN32
    log->document()->setDefaultFont(QFont("Courier New", 10));
#elif __APPLE__
    log->document()->setDefaultFont(QFont("menlo", 10));
#else
    log->document()->setDefaultFont(QFont("monospace", 10));
#endif

    m_log = new LogFormatter(log, this);

    m_tab->addTab(log, QIcon(svgPath + "card-text-outline.svg"), "Log");

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
    Pkg pkg;
    Error newError;

    pkg = pkg_open(pkgPath.c_str(), &newError);

    if (!pkg) {
        QMessageBox::critical(&progress, "Error", QString("Cannot open %1: %2").arg(pkgPath.c_str()).arg(newError.message()));
        return;
    }

    // Get game ID.
    char *error;

    Param param(pkg_get_param(pkg, &error));

    if (!param) {
        QMessageBox::critical(&progress, "Error", QString("Failed to get param.sfo from %1: %2").arg(pkgPath.c_str()).arg(error));
        std::free(error);
        return;
    }

    // Create game directory.
    auto gamesDirectory = readGamesDirectorySetting();

    if (!QDir(gamesDirectory).mkdir(param.titleId())) {
        QString msg(
            "Cannot create directory %1 inside %2. "
            "If you have a failed installation from a previous attempt, you will need to remove this directory before trying again.");

        QMessageBox::critical(&progress, "Error", msg.arg(param.titleId()).arg(gamesDirectory));
        return;
    }

    auto directory = joinPath(gamesDirectory, param.titleId());

    // Extract items.
    progress.setWindowTitle(param.title());

    newError = pkg_extract(pkg, directory.c_str(), [](const char *name, std::uint64_t total, std::uint64_t written, void *ud) {
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

    pkg.close();
    progress.complete();

    if (newError) {
        QMessageBox::critical(this, "Error", QString("Failed to extract %1: %2").arg(pkgPath.c_str()).arg(newError.message()));
        return;
    }

    // Add to game list.
    auto success = loadGame(param.titleId());

    if (success) {
        QMessageBox::information(this, "Success", "Package installed successfully.");
    }
}

void MainWindow::reportIssue()
{
    if (!QDesktopServices::openUrl(QUrl("https://github.com/obhq/obliteration/issues"))) {
        QMessageBox::critical(this, "Error", "Failed to open https://github.com/obhq/obliteration/issues.");
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
    m_log->reset();
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

    args << "--system" << readSystemDirectorySetting();
    args << "--game" << game->directory();
    args << "--debug-dump" << kernelDebugDump();
    args << "--clear-debug-dump";

    // Setup environment variable.
    auto env = QProcessEnvironment::systemEnvironment();

    env.insert("TERM", "xterm");

    // Prepare kernel launching.
    m_kernel = new QProcess(this);
    m_kernel->setProgram(path);
    m_kernel->setArguments(args);
    m_kernel->setProcessEnvironment(env);
    m_kernel->setProcessChannelMode(QProcess::MergedChannels);

    connect(m_kernel, &QProcess::errorOccurred, this, &MainWindow::kernelError);
    connect(m_kernel, &QIODevice::readyRead, this, &MainWindow::kernelOutput);
    connect(m_kernel, &QProcess::finished, this, &MainWindow::kernelTerminated);

    // Launch kernel.
    m_kernel->start(QIODeviceBase::ReadOnly | QIODeviceBase::Text);
}

void MainWindow::kernelError(QProcess::ProcessError error)
{
    // Get error message.
    QString msg;

    switch (error) {
    case QProcess::FailedToStart:
        msg = QString("Failed to launch %1.").arg(m_kernel->program());
        break;
    case QProcess::Crashed:
        msg = "The kernel crashed.";
        break;
    default:
        msg = "The kernel encountered an unknown error.";
    }

    // Flush the kenel log before we destroy its object.
    kernelOutput();

    // Destroy object.
    m_kernel->deleteLater();
    m_kernel = nullptr;

    // Display error.
    QMessageBox::critical(this, "Error", msg);
}

void MainWindow::kernelOutput()
{
    // It is possible for Qt to signal this slot after QProcess::errorOccurred or QProcess::finished
    // so we need to check if the those signals has been received.
    if (!m_kernel) {
        return;
    }

    while (m_kernel->canReadLine()) {
        auto line = QString::fromUtf8(m_kernel->readLine());

        m_log->appendMessage(line);
    }
}

void MainWindow::kernelTerminated(int, QProcess::ExitStatus)
{
    // Do nothing if we got QProcess::errorOccurred before this signal.
    if (!m_kernel) {
        return;
    }

    kernelOutput();

    QMessageBox::critical(this, "Error", "The emulator kernel has stopped unexpectedly. Please take a look at the log and report this issue if possible.");

    m_kernel->deleteLater();
    m_kernel = nullptr;
}

bool MainWindow::loadGame(const QString &gameId)
{
    auto gamesDirectory = readGamesDirectorySetting();
    auto gamePath = joinPath(gamesDirectory, gameId);
    auto gameList = reinterpret_cast<GameListModel *>(m_games->model());

    // Read game title from param.sfo.
    auto paramDir = joinPath(gamePath.c_str(), "sce_sys");
    auto paramPath = joinPath(paramDir.c_str(), "param.sfo");
    Error error;
    Param param(param_open(paramPath.c_str(), &error));

    if (!param) {
        QMessageBox::critical(this, "Error", QString("Cannot open %1: %2").arg(paramPath.c_str()).arg(error.message()));
        return false;
    }

    // Add to list.
    gameList->add(new Game(param.title(), gamePath.c_str()));

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
        QMessageBox::critical(this, "Error", "This function is not available while a game is running.");
        return false;
    }

    return true;
}
