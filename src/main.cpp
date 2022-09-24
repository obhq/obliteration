#include "context.hpp"
#include "initialize_dialog.hpp"
#include "main_window.hpp"
#include "settings.hpp"

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

    // Initialize user settings.
    QApplication app(argc, argv);

    if (!hasRequiredUserSettings()) {
        InitializeDialog init;

        if (!init.exec()) {
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
