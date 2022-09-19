#include "emulator.hpp"
#include "main_window.hpp"

#include <QApplication>
#include <QMessageBox>

#include <cstdlib>

static int run(emulator_t emulator)
{
    MainWindow w(emulator);

    w.show();
    w.reloadGames();

    return QApplication::exec();
}

int main(int argc, char *argv[])
{
    // Setup application.
    QCoreApplication::setOrganizationName("Obliteration");
    QCoreApplication::setApplicationName("Obliteration");
    QCoreApplication::setAttribute(Qt::AA_DisableWindowContextHelpButton);

    // Initialize.
    QApplication app(argc, argv);
    emulator_t emulator;
    char *error;

    emulator = emulator_init(&error);

    if (!emulator) {
        QMessageBox::critical(nullptr, "Fatal Error", QString::asprintf("Failed to initialize emulator: %s", error));
        free(error);
        return 1;
    }

    // Run main window.
    auto status = run(emulator);

    // Shutdown.
    emulator_term(emulator);

    return status;
}
