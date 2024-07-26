#include "main_window.hpp"
#include "game_models.hpp"
#include "launch_settings.hpp"
#include "logs_viewer.hpp"
#include "path.hpp"
#include "pkg_installer.hpp"
#include "resources.hpp"
#include "screen.hpp"
#include "settings.hpp"

#include <QAction>
#include <QApplication>
#include <QCloseEvent>
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
#include <QStackedWidget>
#include <QToolBar>
#include <QUrl>

#include <filesystem>
#include <utility>

MainWindow::MainWindow() :
    m_main(nullptr),
    m_games(nullptr),
    m_launch(nullptr),
    m_screen(nullptr)
{
    setWindowTitle("Obliteration");

    // File menu.
    auto fileMenu = menuBar()->addMenu("&File");
    auto installPkg = new QAction("&Install PKG", this);
    auto openSystemFolder = new QAction("Open System &Folder", this);
    auto quit = new QAction("&Quit", this);

#ifndef __APPLE__
    installPkg->setIcon(loadIcon(":/resources/archive-arrow-down-outline.svg"));
    openSystemFolder->setIcon(loadIcon(":/resources/folder-open-outline.svg"));
#endif

    connect(installPkg, &QAction::triggered, this, &MainWindow::installPkg);
    connect(openSystemFolder, &QAction::triggered, this, &MainWindow::openSystemFolder);
    connect(quit, &QAction::triggered, this, &MainWindow::close);

    fileMenu->addAction(installPkg);
    fileMenu->addAction(openSystemFolder);
    fileMenu->addSeparator();
    fileMenu->addAction(quit);

    // View menu.
    auto viewMenu = menuBar()->addMenu("&View");
    auto logs = new QAction("Logs", this);

    connect(logs, &QAction::triggered, this, &MainWindow::viewLogs);

    viewMenu->addAction(logs);

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

    // Central widget.
    m_main = new QStackedWidget();

    setCentralWidget(m_main);

    // Launch settings.
    m_games = new GameListModel(this);
    m_launch = new LaunchSettings(m_games);

    connect(m_launch, &LaunchSettings::startClicked, this, &MainWindow::startKernel);

    m_main->addWidget(m_launch);

    // Screen.
    m_screen = new Screen();

    m_main->addWidget(createWindowContainer(m_screen));

    // Show the window.
    restoreGeometry();
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

    m_games->sort(0);

    return true;
}

void MainWindow::closeEvent(QCloseEvent *event)
{
    // This will set to accept by QMainWindow::closeEvent.
    event->ignore();

    // Ask user to confirm.
    if (m_kernel) {
        QMessageBox confirm(this);

        confirm.setText("Do you want to exit?");
        confirm.setInformativeText("The running game will be terminated.");
        confirm.setStandardButtons(QMessageBox::Cancel | QMessageBox::Yes);
        confirm.setDefaultButton(QMessageBox::Cancel);
        confirm.setIcon(QMessageBox::Warning);

        if (confirm.exec() != QMessageBox::Yes) {
            return;
        }

        m_kernel.free();
    }

    // Close child windows.
    if (m_logs && !m_logs->close()) {
        return;
    }

    // Save geometry.
    QSettings settings;

    settings.beginGroup(SettingGroups::mainWindow);

    settings.setValue("size", size());
    settings.setValue("maximized", isMaximized());

    if (qGuiApp->platformName() != "wayland") {
        // Wayland does not allow application to position itself.
        settings.setValue("pos", pos());
    }

    QMainWindow::closeEvent(event);
}

void MainWindow::installPkg()
{
    // Browse a PKG.
    auto path = QDir::toNativeSeparators(QFileDialog::getOpenFileName(this, "Install PKG", QString(), "PKG Files (*.pkg)"));

    if (path.isEmpty()) {
        return;
    }

    // Run installer.
    PkgInstaller installer(readGamesDirectorySetting(), path, this);

    if (!installer.exec()) {
        return;
    }

    // Add to game list if new game.
    auto &id = installer.gameId();
    bool success = false;

    if (!id.isEmpty()) {
        success = loadGame(id);
    } else {
        success = true;
    }

    if (success) {
        QMessageBox::information(this, "Success", "Package installed successfully.");
    }
}

void MainWindow::openSystemFolder()
{
    QString folderPath = readSystemDirectorySetting();
    QDesktopServices::openUrl(QUrl::fromLocalFile(folderPath));
}

void MainWindow::viewLogs()
{
    // Check for previous window.
    if (m_logs) {
        m_logs->activateWindow();
        m_logs->raise();
        return;
    }

    // Create a window.
    m_logs = new LogsViewer();

    m_logs->setAttribute(Qt::WA_DeleteOnClose);
    m_logs->show();
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

void MainWindow::startKernel()
{
    // Get full path to kernel binary.
    std::string kernel;

    if (QFile::exists(".obliteration-development")) {
        auto b = std::filesystem::current_path();
#ifdef _WIN32
        auto target = L"x86_64-unknown-none";
#elif defined(__aarch64__)
        auto target = "aarch64-unknown-none-softfloat";
#else
        auto target = "x86_64-unknown-none";
#endif

#if defined(_WIN32) && defined(NDEBUG)
        kernel = (b / L"src" / L"target" / target / L"release" / L"obkrnl").u8string();
#elif defined(_WIN32) && !defined(NDEBUG)
        kernel = (b / L"src" / L"target" / target / L"debug" / L"obkrnl").u8string();
#elif defined(NDEBUG)
        kernel = (b / "src" / "target" / target / "release" / "obkrnl").u8string();
#else
        kernel = (b / "src" / "target" / target / "debug" / "obkrnl").u8string();
#endif
    } else {
#ifdef _WIN32
        std::filesystem::path b(QCoreApplication::applicationDirPath().toStdString(), std::filesystem::path::native_format);
        b /= L"share";
        b /= L"obkrnl";
        kernel = b.u8string();
#else
        auto b = std::filesystem::path(QCoreApplication::applicationDirPath().toStdString(), std::filesystem::path::native_format).parent_path();
#ifdef __APPLE__
        b /= "Resources";
#else
        b /= "share";
#endif
        b /= "obkrnl";
        kernel = b.u8string();
#endif
    }

    // Create VMM.
    RustPtr<RustError> error;
    RustPtr<Vmm> vmm;

    vmm = vmm_new(&error);

    if (!vmm) {
        QMessageBox::critical(
            this,
            "Error",
            QString("Couldn't create a VMM: %1").arg(error_message(error)));
        return;
    }

    // Run.
    error = vmm_run(vmm, kernel.c_str());

    if (error) {
        QMessageBox::critical(
            this,
            "Error",
            QString("Couldn't run %1: %2").arg(kernel.c_str()).arg(error_message(error)));
        return;
    }

    m_kernel = std::move(vmm);
    m_main->setCurrentIndex(1);
}

bool MainWindow::loadGame(const QString &gameId)
{
    auto gamesDirectory = readGamesDirectorySetting();
    auto gamePath = joinPath(gamesDirectory, gameId);

    // Ignore entry if it is DLC or Patch.
    auto lastSlashPos = gamePath.find_last_of("/\\");
    auto lastFolder = (lastSlashPos != std::string::npos) ? gamePath.substr(lastSlashPos + 1) : gamePath;
    bool isPatch = lastFolder.find("-PATCH-") != std::string::npos;
    bool isAddCont = lastFolder.size() >= 8 && lastFolder.substr(lastFolder.size() - 8) == "-ADDCONT";

    if (!isPatch && !isAddCont) {

        // Read game information from param.sfo.
        auto paramDir = joinPath(gamePath.c_str(), "sce_sys");
        auto paramPath = joinPath(paramDir.c_str(), "param.sfo");
        RustPtr<RustError> error;
        RustPtr<Param> param;

        param = param_open(paramPath.c_str(), &error);

        if (!param) {
            QMessageBox::critical(
                this,
                "Error",
                QString("Cannot open %1: %2").arg(paramPath.c_str()).arg(error_message(error)));
            return false;
        }

        // Add to list.
        RustPtr<char> titleId, title;

        titleId = param_title_id_get(param);
        title = param_title_get(param);

        m_games->add(new Game(titleId.get(), title.get(), gamePath.c_str()));
    }

    return true;
}

void MainWindow::restoreGeometry()
{
    QSettings settings;

    settings.beginGroup(SettingGroups::mainWindow);

    if (settings.value("maximized", false).toBool()) {
        showMaximized();
    } else {
        resize(settings.value("size", QSize(1000, 500)).toSize());

        if (qGuiApp->platformName() != "wayland") {
            move(settings.value("pos", QPoint(200, 200)).toPoint());
        }

        show();
    }
}

bool MainWindow::requireEmulatorStopped()
{
    if (m_kernel) {
        QMessageBox killPrompt(this);

        killPrompt.setText("Action requires kernel to be stopped to continue.");
        killPrompt.setInformativeText("Do you want to kill the kernel?");
        killPrompt.setStandardButtons(QMessageBox::Cancel | QMessageBox::Yes);
        killPrompt.setDefaultButton(QMessageBox::Cancel);
        killPrompt.setIcon(QMessageBox::Warning);

        if (killPrompt.exec() != QMessageBox::Yes) {
            return true;
        }

        m_kernel.free();
    }

    return false;
}
