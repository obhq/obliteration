#include "initialize_wizard.hpp"
#include "main_window.hpp"
#include "settings.hpp"
#include "system.hpp"

#include <QApplication>
#include <QMessageBox>

#include <cstdlib>

int main(int argc, char *argv[])
{
    // Setup application.
    QCoreApplication::setOrganizationName("Obliteration");
    QCoreApplication::setApplicationName("Obliteration");

    // Dark Mode for Windows
    #ifdef _WIN32
        QApplication::setStyle("Fusion");
    #endif

    QApplication app(argc, argv);

    QGuiApplication::setWindowIcon(QIcon(":/resources/obliteration-icon.png"));

    // Check if no any required settings.
    if (!hasRequiredUserSettings() || !isSystemInitialized()) {
        InitializeWizard init;

        if (!init.exec()) {
            return 1;
        }
    }

    if (!ensureSystemDirectories()) {
        return 1;
    }

    // Run main window.
    MainWindow win;

    win.show();

    if (!win.loadGames()) {
        return 1;
    }

    return QApplication::exec();
}
