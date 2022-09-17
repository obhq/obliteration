#include "initialize_dialog.hpp"
#include "game_models.hpp"
#include "main_window.hpp"
#include "settings.hpp"

#include <QApplication>
#include <QDir>
#include <QProgressDialog>

static bool init(GameListModel &games)
{
    // Get game counts.
    QDir gamesDirectory(readGamesDirectorySetting());
    auto gameDirectories = gamesDirectory.entryList(QDir::Dirs | QDir::NoDotAndDotDot);

    // Setup loading progress.
    int step = -1;
    QProgressDialog progress;

    progress.setMaximum(gameDirectories.size());
    progress.setCancelButtonText("Cancel");
    progress.setWindowModality(Qt::WindowModal);
    progress.setValue(++step);

    // Load games
    progress.setLabelText("Loading games...");

    for (auto &dir : gameDirectories) {
        if (progress.wasCanceled()) {
            return false;
        }

        games.add(new Game(dir));
        progress.setValue(++step);
    }

    return true;
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

    if (!init(games)) {
        return 1;
    }

    // Show main window.
    MainWindow win(&games);

    win.show();

    return app.exec();
}
