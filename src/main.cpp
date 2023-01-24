#include "context.hpp"
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

static int run(context *context)
{
    MainWindow w(context);

    w.show();

    if (!w.loadGames()) {
        return 1;
    }

    return QApplication::exec();
}

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

    // Initialize system.
    context *context;
    char *error;

    context = context_new(&error);

    if (!context) {
        QMessageBox::critical(nullptr, "Fatal Error", QString("Failed to initialize application system: %1").arg(error));
        std::free(error);
        return 1;
    }

    // Run main window.
    auto status = run(context);

    // Shutdown.
    context_free(context);

    return status;
}
