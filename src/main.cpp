#include "initialize_dialog.hpp"
#include "emulator.hpp"
#include "game_models.hpp"
#include "main_window.hpp"
#include "settings.hpp"

#include <QApplication>
#include <QDir>
#include <QMessageBox>
#include <QProgressDialog>

#include <cstdlib>

static void *init(GameListModel &games)
{
    // Get game counts.
    QDir gamesDirectory(readGamesDirectorySetting());
    auto gameDirectories = gamesDirectory.entryList(QDir::Dirs | QDir::NoDotAndDotDot);

    // Setup loading progress.
    int step = -1;
    QProgressDialog progress;

    progress.setMaximum(gameDirectories.size() + 1);
    progress.setCancelButtonText("Cancel");
    progress.setWindowModality(Qt::WindowModal);
    progress.setValue(++step);

    // Load games
    progress.setLabelText("Loading games...");

    for (auto &dir : gameDirectories) {
        if (progress.wasCanceled()) {
            return nullptr;
        }

        games.add(new Game(dir));
        progress.setValue(++step);
    }

    // Initialize emulator system.
    void *emulator;
    char *error;

    emulator = emulator_init(&error);

    if (!emulator) {
        QMessageBox::critical(&progress, "Fatal Error", QString::asprintf("Failed to initialize emulator: %s", error));
        free(error);
        return nullptr;
    }

    progress.setValue(++step);

    return emulator;
}

static int run(GameListModel *games)
{
    MainWindow win(games);

    win.show();

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

    // Initialize application.
    GameListModel games;
    auto emulator = init(games);

    if (!emulator) {
        return 1;
    }

    // Run main window.
    auto status = run(&games);

    // Shutdown.
    emulator_term(emulator);

    return status;
}
