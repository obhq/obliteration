#include "system.hpp"
#include "path.hpp"
#include "progress_dialog.hpp"
#include "settings.hpp"
#include "string.hpp"
#include "system_downloader.hpp"

#include <QCoreApplication>
#include <QDir>
#include <QMessageBox>
#include <QThread>

bool isSystemInitialized()
{
    return isSystemInitialized(readSystemDirectorySetting());
}

bool isSystemInitialized(const QString &path)
{
    auto libkernel = toPath(path);

    try {
        libkernel /= STR("system");
        libkernel /= STR("common");
        libkernel /= STR("lib");
        libkernel /= STR("libkernel.sprx");

        return std::filesystem::exists(libkernel);
    } catch (...) {
        return false;
    }
}

bool initSystem(const QString &path, const QString &from, bool explicitDecryption, QWidget *parent)
{
    // Setup progress dialog.
    ProgressDialog progress("Initializing system", QString("Connecting to %1").arg(from), parent);

    // Setup the system downloader.
    QThread background;
    QObject context;
    QString error;
    auto finished = false;
    auto downloader = new SystemDownloader(from, path, explicitDecryption);

    downloader->moveToThread(&background);

    QObject::connect(&background, &QThread::started, downloader, &SystemDownloader::exec);
    QObject::connect(&background, &QThread::finished, downloader, &QObject::deleteLater);

    QObject::connect(downloader, &SystemDownloader::statusChanged, &context, [&](auto status, auto total, auto written) {
        if (progress.statusText() != status) {
            progress.setStatusText(status);
            progress.setValue(0);
            progress.setMaximum(total);
        } else {
            progress.setValue(written);
        }
    });

    QObject::connect(downloader, &SystemDownloader::finished, &context, [&](auto e) {
        error = e;
        finished = true;
    });

    // Start dumping.
    background.start();

    while (!finished) {
        QCoreApplication::processEvents(QEventLoop::WaitForMoreEvents);
    }

    // Clean up.
    background.quit();
    background.wait();
    progress.complete();

    // Check result.
    if (!error.isEmpty()) {
        QMessageBox::critical(parent, "Error", QString("Failed to download system files from %1 to %2: %3").arg(from).arg(path).arg(error));
        return false;
    }

    QMessageBox::information(parent, "Success", "Downloaded system files successfully.");

    return true;
}
