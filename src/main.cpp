#include "initialize_wizard.hpp"
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

    // Check if no any required settings.
    if (!hasRequiredUserSettings() || !hasSystemFilesInstalled()) {
        InitializeWizard init;

        if (!init.exec()) {
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
