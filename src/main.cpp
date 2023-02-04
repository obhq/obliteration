#include "initialize_dialog.hpp"
#include "main_window.hpp"
#include "settings.hpp"
#include "system.hpp"
#ifdef Q_OS_WIN
    #include "darkmode.hpp"
#endif

#include <QApplication>
#include <QMessageBox>

#include <cstdlib>

int main(int argc, char *argv[])
{
    // Setup application.
    QCoreApplication::setOrganizationName("Obliteration");
    QCoreApplication::setApplicationName("Obliteration");

    QApplication app(argc, argv);

    QGuiApplication::setWindowIcon(QIcon(":/resources/obliteration-icon.png"));

    // Set dark/light mode for Windows, Qt already handles MacOS and Linux themes.
    #ifdef Q_OS_WIN
        set_darkmode();
    #endif

    // Initialize user settings.
    if (!hasRequiredUserSettings()) {
        InitializeDialog init;

        if (!init.exec()) {
            return 1;
        }
    }

    // Install system files.
    if (!hasSystemFilesInstalled()) {
        if (!updateSystemFiles(nullptr)) {
            return 1;
        }
    }

    // Run main window.
    MainWindow win;

    win.show();

    if (!win.loadGames()) {
        return 1;
    }

    return QApplication::exec();
}
