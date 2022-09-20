#include "emulator.hpp"
#include "initialize_dialog.hpp"
#include "main_window.hpp"
#include "settings.hpp"

#include <QApplication>
#include <QMessageBox>

#include <cstdlib>

static int run(context_t context)
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
    QCoreApplication::setAttribute(Qt::AA_DisableWindowContextHelpButton);

    // Initialize user settings.
    QApplication app(argc, argv);

    if (!hasRequiredUserSettings()) {
        InitializeDialog init;

        if (!init.exec()) {
            return 1;
        }
    }

    // Initialize system.
    context_t context;
    char *error;

    context = emulator_init(&error);

    if (!context) {
        QMessageBox::critical(nullptr, "Fatal Error", QString("Failed to initialize emulator: %1").arg(error));
        std::free(error);
        return 1;
    }

    // Run main window.
    auto status = run(context);

    // Shutdown.
    emulator_term(context);

    return status;
}
