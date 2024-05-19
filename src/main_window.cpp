#include "main_window.hpp"
#include "app_data.hpp"
#include "core.hpp"
#include "game_models.hpp"
#include "game_settings.hpp"
#include "game_settings_dialog.hpp"
#include "log_formatter.hpp"
#include "path.hpp"
#include "pkg_installer.hpp"
#include "settings.hpp"
#include "string.hpp"

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
#include <QResizeEvent>
#include <QScrollBar>
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

    // Determine current theme.
    QString svgPath;

    if (QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark) {
        svgPath = ":/resources/darkmode/";
    } else {
        svgPath = ":/resources/lightmode/";
    }

    // File menu.
    auto fileMenu = menuBar()->addMenu("&File");
    auto installPkg = new QAction("&Install PKG", this);
    auto openSystemFolder = new QAction("Open System &Folder", this);
    auto quit = new QAction("&Quit", this);

#ifndef __APPLE__
    installPkg->setIcon(QIcon(svgPath + "archive-arrow-down-outline.svg"));
    openSystemFolder->setIcon(QIcon(svgPath + "folder-open-outline.svg"));
#endif

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
    m_games->setContextMenuPolicy(Qt::CustomContextMenu);
    m_games->setLayoutMode(QListView::SinglePass);
    m_games->setModel(new GameListModel(this));
    m_games->setViewMode(QListView::IconMode);
    m_games->setWordWrap(true);
    m_games->verticalScrollBar()->setSingleStep(20);


    connect(m_games, &QAbstractItemView::doubleClicked, this, &MainWindow::startGame);
    connect(m_games, &QWidget::customContextMenuRequested, this, &MainWindow::requestGamesContextMenu);

    m_tab->addTab(m_games, QIcon(svgPath + "view-comfy.svg"), "Games");

    connect(m_tab, &QTabWidget::currentChanged, this, &MainWindow::tabChanged);

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
    auto gameList = reinterpret_cast<GameListModel *>(m_games->model());

    for (auto &gameId : games) {
        if (progress.wasCanceled() || !loadGame(gameId)) {
            return false;
        }

        progress.setValue(++step);
    }

    gameList->sort(0, Qt::AscendingOrder); // TODO add ability to select descending order (button?)

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

void MainWindow::resizeEvent(QResizeEvent *event)
{
    // Allows the games list to resort if window is resized.
    if (m_tab->currentIndex() == 0) {
        m_games->updateGeometry();
        m_games->doItemsLayout();
    }

    QMainWindow::resizeEvent(event);
}

void MainWindow::tabChanged()
{
    // Update games list if window was resized on another tab.
    if (m_tab->currentIndex() == 0) {
        m_games->updateGeometry();
        m_games->doItemsLayout();
    }
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
    QAction openFolder("Open &Folder", this);
    QAction settings("&Settings", this);

#ifndef __APPLE__
    QString svgPath;

    if (QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark) {
        svgPath = ":/resources/darkmode/";
    } else {
        svgPath = ":/resources/lightmode/";
    }

    openFolder.setIcon(QIcon(svgPath + "folder-open-outline.svg"));
    settings.setIcon(QIcon(svgPath + "cog-outline.svg"));
#endif

    menu.addAction(&openFolder);
    menu.addAction(&settings);

    // Show menu.
    auto selected = menu.exec(m_games->viewport()->mapToGlobal(pos));

    if (!selected) {
        return;
    }

    if (selected == &openFolder) {
        QString folderPath = game->directory();
        QDesktopServices::openUrl(QUrl::fromLocalFile(folderPath));
    } else if (selected == &settings) {
        // Load settings then show a dialog to edit.
        auto settings = GameSettings::load(game);
        GameSettingsDialog dialog(game, settings.get(), this);

        dialog.exec();
    }
}

void MainWindow::startGame(const QModelIndex &index)
{
    if (requireEmulatorStopped()) {
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

    args << "--debug-dump" << kernelDebugDump();
    args << "--clear-debug-dump";
    args << readSystemDirectorySetting();
    args << game->directory();

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

    // Ignore entry if it is DLC or Patch.
    auto lastSlashPos = gamePath.find_last_of("/\\");
    auto lastFolder = (lastSlashPos != std::string::npos) ? gamePath.substr(lastSlashPos + 1) : gamePath;
    bool isPatch = lastFolder.find("-PATCH-") != std::string::npos;
    bool isAddCont = lastFolder.size() >= 8 && lastFolder.substr(lastFolder.size() - 8) == "-ADDCONT";

    if (!isPatch && !isAddCont) {

        // Read game information from param.sfo.
        auto paramDir = joinPath(gamePath.c_str(), "sce_sys");
        auto paramPath = joinPath(paramDir.c_str(), "param.sfo");
        Error error;
        Param param(param_open(paramPath.c_str(), &error));

        if (!param) {
            QMessageBox::critical(this, "Error", QString("Cannot open %1: %2").arg(paramPath.c_str()).arg(error.message()));
            return false;
        }

        // Add to list.
        auto list = reinterpret_cast<GameListModel *>(m_games->model());

        list->add(new Game(param.titleId(), param.title(), gamePath.c_str()));
    }

    return true;
}

void MainWindow::killKernel()
{
    // Do nothing if the kernel already terminated. This prevent a crash if this method is putting
    // behind the message box and the kernel itself was terminated while waiting for the user to confirm.
    if (!m_kernel) {
        return;
    }

    // We need to disconnect all slots first otherwise the application will be freeze.
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
        if (killPrompt.exec() == QMessageBox::Yes) {
            killKernel();
            return false; // Kernel was killed
        }

        return true; // Kernel left running
    }

    return false; // Kernel isn't running
}
