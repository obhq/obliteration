#include "initialize_dialog.hpp"
#include "main_window.hpp"
#include "settings.hpp"

#include <QApplication>

int main(int argc, char *argv[])
{
    // Setup application.
    QApplication app(argc, argv);

    QCoreApplication::setOrganizationName("Obliteration");
    QCoreApplication::setApplicationName("Obliteration");

    // Initialize user settings.
    if (!hasRequiredUserSettings()) {
        InitializeDialog init;

        if (init.exec() != QDialog::Accepted) {
            return 1;
        }
    }

    // Create main window.
    MainWindow win;

    win.show();

    return app.exec();
}
