#include "initialize_wizard.hpp"
#include "main_window.hpp"
#include "settings.hpp"
#include "system.hpp"

#include <QApplication>
#include <QMessageBox>

#include <cstdlib>

#ifndef _WIN32
#include <sys/resource.h>
#endif

int main(int argc, char *argv[])
{
    // Setup application.
    QCoreApplication::setOrganizationName("OBHQ");
    QCoreApplication::setApplicationName("Obliteration");
    QApplication::setStyle("Fusion");

    QApplication app(argc, argv);

    QGuiApplication::setWindowIcon(QIcon(":/resources/obliteration-icon.png"));

    // Increase number of file descriptors to maximum allowed.
#ifndef _WIN32
    rlimit limit;

    if (getrlimit(RLIMIT_NOFILE, &limit) == 0) {
        if (limit.rlim_cur < limit.rlim_max) {
            limit.rlim_cur = limit.rlim_max;

            if (setrlimit(RLIMIT_NOFILE, &limit) < 0) {
                QMessageBox::warning(
                    nullptr,
                    "Warning",
                    "Failed to set file descriptor limit to maximum allowed.");
            }
        }
    } else {
        QMessageBox::warning(nullptr, "Warning", "Failed to get file descriptor limit.");
    }
#endif

    // Check if no any required settings.
    if (!hasRequiredUserSettings() || !isSystemInitialized()) {
        InitializeWizard init;

        if (!init.exec()) {
            return 1;
        }
    }

    // Run main window.
    MainWindow win;

    if (!win.loadGames()) {
        return 1;
    }

    return QApplication::exec();
}
