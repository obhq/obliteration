#include "main_window.hpp"
#include "app_data.hpp"
#include "game_models.hpp"
#include "launch_settings.hpp"
#include "path.hpp"
#include "pkg_installer.hpp"
#include "profile_models.hpp"
#include "resources.hpp"
#include "screen.hpp"
#include "settings.hpp"

#include <QAction>
#include <QApplication>
#include <QCloseEvent>
#include <QCommandLineOption>
#include <QCommandLineParser>
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
#include <QSocketNotifier>
#include <QStackedWidget>
#include <QToolBar>
#include <QUrl>

#include <filesystem>
#include <utility>

#ifndef _WIN32
#include <fcntl.h>
#endif

namespace Args {
    const QCommandLineOption debug("debug", "Immediate launch the VMM in debug mode.", "addr", "127.0.0.1:1234");
    const QCommandLineOption kernel("kernel", "Use this kernel instead of default one.", "path");
}

#ifdef __APPLE__
MainWindow::MainWindow(const QCommandLineParser &args) :
#else
MainWindow::MainWindow(
    const QCommandLineParser &args,
    QVulkanInstance *vulkan,
    QList<VkPhysicalDevice> &&vkDevices) :
#endif
    m_args(args),
    m_main(nullptr),
    m_profiles(nullptr),
    m_games(nullptr),
    m_launch(nullptr),
    m_screen(nullptr),
    m_debugNoti(nullptr)
{
    setWindowTitle("Obliteration");

    // File menu.
    auto fileMenu = menuBar()->addMenu("&File");
    auto installPkg = new QAction("&Install PKG", this);
    auto openSystemFolder = new QAction("Open System &Folder", this);
    auto quit = new QAction("&Quit", this);

    connect(installPkg, &QAction::triggered, this, &MainWindow::installPkg);
    connect(openSystemFolder, &QAction::triggered, this, &MainWindow::openSystemFolder);
    connect(quit, &QAction::triggered, this, &MainWindow::close);

    fileMenu->addAction(installPkg);
    fileMenu->addAction(openSystemFolder);
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

    // Central widget.
    m_main = new QStackedWidget();

    setCentralWidget(m_main);

    // Launch settings.
    m_profiles = new ProfileList(this);
    m_games = new GameListModel(this);
#ifdef __APPLE__
    m_launch = new LaunchSettings(m_profiles, m_games);
#else
    m_launch = new LaunchSettings(m_profiles, m_games, std::move(vkDevices));
#endif

    connect(m_launch, &LaunchSettings::saveClicked, this, &MainWindow::saveProfile);
    connect(m_launch, &LaunchSettings::startClicked, [this](const QString &debug) {
        if (debug.isEmpty()) {
            startVmm({});
        } else {
            startDebug(debug);
        }
    });

    m_main->addWidget(m_launch);

    // Screen.
    m_screen = new Screen();

#ifndef __APPLE__
    m_screen->setVulkanInstance(vulkan);
#endif

    m_main->addWidget(createWindowContainer(m_screen));
}

MainWindow::~MainWindow()
{
}

bool MainWindow::loadProfiles()
{
    // List profile directories.
    auto root = profiles();
    auto dirs = QDir(root).entryList(QDir::Dirs | QDir::NoDotAndDotDot);

    // Create default profile if the user don't have any profiles.
    if (dirs.isEmpty()) {
        Rust<Profile> p;
        Rust<char> id;

        p = profile_new("Default");
        id = profile_id(p);

        // Save.
        auto path = joinPath(root, id.get());
        Rust<RustError> error;

        error = profile_save(p, path.c_str());

        if (error) {
            auto text = QString("Failed to save default profile to %1: %2.")
                .arg(path.c_str())
                .arg(error_message(error));

            QMessageBox::critical(this, "Error", text);
            return false;
        }

        dirs.append(id.get());
    }

    // Load profiles.
    for (auto &dir : dirs) {
        auto path = joinPath(root, dir);
        Rust<RustError> error;
        Rust<Profile> profile;

        profile = profile_load(path.c_str(), &error);

        if (!profile) {
            auto text = QString("Failed to load a profile from %1: %2.")
                .arg(path.c_str())
                .arg(error_message(error));

            QMessageBox::critical(this, "Error", text);
            return false;
        }

        m_profiles->add(std::move(profile));
    }

    return true;
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
    if (m_vmm) {
        // Ask user to confirm only if we did not shutdown the VMM programmatically.
        if (!vmm_shutting_down(m_vmm)) {
            QMessageBox confirm(this);

            confirm.setText("Do you want to exit?");
            confirm.setInformativeText("The running game will be terminated.");
            confirm.setStandardButtons(QMessageBox::Cancel | QMessageBox::Yes);
            confirm.setDefaultButton(QMessageBox::Cancel);
            confirm.setIcon(QMessageBox::Warning);

            if (confirm.exec() != QMessageBox::Yes) {
                return;
            }
        }

        killVmm();
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

void MainWindow::reportIssue()
{
    if (!QDesktopServices::openUrl(QUrl("https://github.com/obhq/obliteration/issues/new"))) {
        QMessageBox::critical(this, "Error", "Failed to open https://github.com/obhq/obliteration/issues/new.");
    }
}

void MainWindow::aboutObliteration()
{
    QMessageBox::about(
        this,
        "About Obliteration",
        "Obliteration is a free and open-source PlayStation 4 kernel. It will allows you to run "
        "the PlayStation 4 system software that you have dumped from your PlayStation 4 on your "
        "PC. This will allows you to play your games forever even if your PlayStation 4 stopped "
        "working in the future.");
}

void MainWindow::saveProfile(Profile *p)
{
    // Get ID.
    Rust<char> id;

    id = profile_id(p);

    // Save.
    auto root = profiles();
    auto path = joinPath(root, id.get());
    Rust<RustError> error;

    error = profile_save(p, path.c_str());

    if (error) {
        auto text = QString("Failed to save %1 profile to %2: %3.")
            .arg(profile_name(p))
            .arg(path.c_str())
            .arg(error_message(error));

        QMessageBox::critical(this, "Error", text);
    }
}

void MainWindow::debuggerConnected()
{
    // Free QSocketNotifier.
    delete m_debugNoti;
    m_debugNoti = nullptr;

    // Accept a connection.
    Rust<RustError> error;
    Rust<DebugClient> debugger;

    debugger = debug_server_accept(m_debugServer, &error);

    m_debugServer.free();

    if (!debugger) {
        QMessageBox::critical(
            this,
            "Error",
            QString("Failed to accept a debugger connection: %1.").arg(error_message(error)));
        return;
    }

    // Start VMM.
    startVmm(std::move(debugger));
}

void MainWindow::vmmError(const QString &msg)
{
    killVmm();

    QMessageBox::critical(this, "Error", msg);

    if (m_args.isSet(Args::debug)) {
        close();
    } else {
        m_main->setCurrentIndex(0);
    }
}

void MainWindow::waitKernelExit(bool success)
{
    killVmm();

    if (!success) {
        QMessageBox::critical(
            this,
            "Error",
            "The kernel was stopped unexpectedly. See the kernel logs for more details.");
    }

    m_main->setCurrentIndex(0);
}

void MainWindow::setupDebugger()
{
    // Enable non-blocking on debug socket. On Windows QSocketNotifier will do this for us.
    auto sock = vmm_debug_socket(m_vmm);

#ifndef _WIN32
    auto flags = fcntl(sock, F_GETFL);

    if (flags < 0) {
        QMessageBox::critical(
            this,
            "Error",
            "Couldn't get file flags from debug socket.");
        stopDebug();
        return;
    } else if (fcntl(sock, F_SETFL, flags | O_NONBLOCK) < 0) {
        QMessageBox::critical(
            this,
            "Error",
            "Couldn't enable non-blocking mode on debug socket.");
        stopDebug();
        return;
    }
#endif

    // Watch for incoming data.
    m_debugNoti = new QSocketNotifier(QSocketNotifier::Read, this);
    m_debugNoti->setSocket(sock);

    connect(m_debugNoti, &QSocketNotifier::activated, [this] { dispatchDebug(nullptr); });

    m_debugNoti->setEnabled(true);

    // Setup GDB session.
    dispatchDebug(nullptr);
}

void MainWindow::dispatchDebug(KernelStop *stop)
{
    // Do nothing if the previous thread already trigger the shutdown.
    if (vmm_shutting_down(m_vmm)) {
        return;
    }

    // Dispatch debug events.
    auto r = vmm_dispatch_debug(m_vmm, stop);

    switch (r.tag) {
    case DebugResult_Ok:
        break;
    case DebugResult_Disconnected:
        // It is not safe to let the kernel running since it is assume there are a debugger.
        vmm_shutdown(m_vmm);
        break;
    case DebugResult_Error:
        {
            Rust<RustError> e(r.error.reason);

            QMessageBox::critical(
                this,
                "Error",
                QString("Failed to dispatch debug events: %1").arg(error_message(e)));
        }

        vmm_shutdown(m_vmm);
        break;
    }

    if (vmm_shutting_down(m_vmm)) {
        stopDebug();
    }
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
        Rust<RustError> error;
        Rust<Param> param;

        param = param_open(paramPath.c_str(), &error);

        if (!param) {
            QMessageBox::critical(
                this,
                "Error",
                QString("Cannot open %1: %2").arg(paramPath.c_str()).arg(error_message(error)));
            return false;
        }

        // Add to list.
        Rust<char> titleId, title;

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

void MainWindow::startDebug(const QString &addr)
{
    // Start debug server.
    Rust<RustError> error;

    m_debugServer = debug_server_start(addr.toStdString().c_str(), &error);

    if (!m_debugServer) {
        auto msg = QString("Failed to start a debug server on %1: %2")
            .arg(addr)
            .arg(error_message(error));

        QMessageBox::critical(this, "Error", msg);
        return;
    }

    // Watch for connection.
    m_debugNoti = new QSocketNotifier(QSocketNotifier::Read, this);
    m_debugNoti->setSocket(debug_server_socket(m_debugServer));

    connect(
        m_debugNoti,
        &QSocketNotifier::activated,
        this,
        &MainWindow::debuggerConnected,
        Qt::SingleShotConnection);

    m_debugNoti->setEnabled(true);

    // Swap launch settings with the screen now to prevent user update settings.
    m_main->setCurrentIndex(1);

    // Tell the user to connect a debugger.
    if (!m_args.isSet(Args::debug)) {
        QMessageBox::information(
            this,
            "Debug",
            QString("Waiting for a debugger at %1.").arg(debug_server_addr(m_debugServer)));
    }
}

void MainWindow::startVmm(Rust<DebugClient> &&debug)
{
    // Get full path to kernel binary.
    std::string kernel;

    m_debugServer.free();

    if (m_args.isSet(Args::kernel)) {
        kernel = m_args.value(Args::kernel).toStdString();
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

    // Swap launch settings with the screen before getting a Vulkan surface otherwise it will fail.
    m_main->setCurrentIndex(1);
}

bool MainWindow::requireVmmStopped()
{
    if (m_vmm) {
        QMessageBox prompt(this);

        prompt.setText("Action requires VMM to be stopped to continue.");
        prompt.setInformativeText("Do you want to kill the VMM?");
        prompt.setStandardButtons(QMessageBox::Cancel | QMessageBox::Yes);
        prompt.setDefaultButton(QMessageBox::Cancel);
        prompt.setIcon(QMessageBox::Warning);

        if (prompt.exec() != QMessageBox::Yes) {
            return false;
        }

        killVmm();
    }

    return true;
}

void MainWindow::stopDebug()
{
    // We can't free the VMM here because the thread that trigger this method are waiting
    // for us to return.
    if (m_args.isSet(Args::debug)) {
        QMetaObject::invokeMethod(
            this,
            &MainWindow::close,
            Qt::QueuedConnection);
    } else {
        QMetaObject::invokeMethod(
            this,
            &MainWindow::waitKernelExit,
            Qt::QueuedConnection,
            true);
    }
}

void MainWindow::killVmm()
{
    m_vmm.free();

    delete m_debugNoti;
    m_debugNoti = nullptr;
}
