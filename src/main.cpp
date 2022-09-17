#include "initialize_dialog.hpp"
#include "main_window.hpp"
#include "settings.hpp"

#include <QApplication>

int main(int argc, char *argv[])
{
    // Setup application.
    QCoreApplication::setOrganizationName("Obliteration");
    QCoreApplication::setApplicationName("Obliteration");
    QCoreApplication::setAttribute(Qt::AA_EnableHighDpiScaling);
    QCoreApplication::setAttribute(Qt::AA_DisableWindowContextHelpButton);

    // Initialize user settings.
    QApplication app(argc, argv);

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
