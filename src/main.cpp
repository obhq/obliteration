#include "core.hpp"
#include "initialize_wizard.hpp"
#include "main_window.hpp"
#include "settings.hpp"
#include "system.hpp"
#ifndef __APPLE__
#include "vulkan.hpp"
#endif

#include <QApplication>
#include <QMessageBox>
#include <QMetaObject>
#ifndef __APPLE__
#include <QVersionNumber>
#include <QVulkanInstance>
#endif

#ifndef _WIN32
#include <sys/resource.h>
#endif

static void panicHook(
    const char *file,
    size_t flen,
    uint32_t line,
    const char *msg,
    size_t mlen,
    void *cx)
{
    QMetaObject::invokeMethod(reinterpret_cast<QObject *>(cx), [=]() {
        auto text = QString("An unexpected error occurred at %1:%2: %3")
            .arg(QString::fromUtf8(file, flen))
            .arg(line)
            .arg(QString::fromUtf8(msg, mlen));

        QMessageBox::critical(nullptr, "Fatal Error", text);
    });
}

int main(int argc, char *argv[])
{
    // Setup application.
    QCoreApplication::setOrganizationName("OBHQ");
    QCoreApplication::setApplicationName("Obliteration");
    QApplication::setStyle("Fusion");

    QApplication app(argc, argv);

    QGuiApplication::setWindowIcon(QIcon(":/resources/obliteration-icon.png"));

    // Hook Rust panic.
    QObject panic;

    set_panic_hook(&panic, panicHook);

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

    // Initialize Vulkan.
#ifndef __APPLE__
    QVulkanInstance vulkan;

    vulkan.setApiVersion(QVersionNumber(1, 3));

    if (!vulkan.create()) {
        QMessageBox::critical(
            nullptr,
            "Error",
            QString("Failed to initialize Vulkan (%1)").arg(vulkan.errorCode()));
        return 1;
    }

    vkFunctions = vulkan.functions();
#endif

    // Check if no any required settings.
    if (!hasRequiredUserSettings() || !isSystemInitialized()) {
        InitializeWizard init;

        if (!init.exec()) {
            return 1;
        }
    }

    // Run main window.
#ifdef __APPLE__
    MainWindow win;
#else
    MainWindow win(&vulkan);
#endif

    if (!win.loadGames()) {
        return 1;
    }

    return QApplication::exec();
}
