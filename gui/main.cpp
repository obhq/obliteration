#include "initialize_wizard.hpp"
#include "main_window.hpp"
#include "settings.hpp"
#include "system.hpp"
#ifndef __APPLE__
#include "vulkan.hpp"
#endif

#include <QApplication>
#include <QCommandLineParser>
#include <QList>
#include <QMessageBox>
#include <QMetaObject>
#include <QThread>
#ifndef __APPLE__
#include <QVersionNumber>
#include <QVulkanFunctions>
#include <QVulkanInstance>
#endif

#include <utility>

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
    auto main = reinterpret_cast<QObject *>(cx);
    auto type = QThread::currentThread() == main->thread()
        ? Qt::DirectConnection
        : Qt::BlockingQueuedConnection;

    QMetaObject::invokeMethod(main, [=]() {
        auto text = QString("An unexpected error occurred at %1:%2: %3")
            .arg(QString::fromUtf8(file, flen))
            .arg(line)
            .arg(QString::fromUtf8(msg, mlen));

        QMessageBox::critical(nullptr, "Fatal Error", text);
    }, type);
}

int main(int argc, char *argv[])
{
    // Setup application.
    QCoreApplication::setOrganizationName("OBHQ");
    QCoreApplication::setApplicationName("Obliteration");
    QApplication::setStyle("Fusion");

    QApplication app(argc, argv);

    QGuiApplication::setWindowIcon(QIcon(":/resources/obliteration-icon.png"));

    // Parse arguments.
    QCommandLineParser args;

    args.setApplicationDescription("Virtualization stack for Obliteration");
    args.addHelpOption();
    args.addOption(Args::debug);
    args.process(app);

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

#if !defined(NDEBUG)
    vulkan.setLayers({"VK_LAYER_KHRONOS_validation"});
#endif

    if (!vulkan.create()) {
        QMessageBox::critical(
            nullptr,
            "Error",
            QString("Failed to initialize Vulkan (%1).").arg(vulkan.errorCode()));
        return 1;
    }

    vkFunctions = vulkan.functions();

    // List available devices.
    QList<VkPhysicalDevice> vkDevices;

    for (;;) {
        // Get device count.
        uint32_t count;
        auto result = vkFunctions->vkEnumeratePhysicalDevices(vulkan.vkInstance(), &count, nullptr);

        if (result != VK_SUCCESS) {
            QMessageBox::critical(
                nullptr,
                "Error",
                QString("Failed to get a number of Vulkan physical device (%1).").arg(result));
            return 1;
        } else if (!count) {
            QMessageBox::critical(
                nullptr,
                "Error",
                "No any Vulkan physical device available.");
            return 1;
        }

        // Get devices.
        vkDevices.resize(count);

        result = vkFunctions->vkEnumeratePhysicalDevices(
            vulkan.vkInstance(),
            &count,
            vkDevices.data());

        if (result == VK_INCOMPLETE) {
            continue;
        } else if (result != VK_SUCCESS) {
            QMessageBox::critical(
                nullptr,
                "Error",
                QString("Failed to list Vulkan physical devices (%1).").arg(result));
            return 1;
        }

        break;
    }

    // Filter out devices without Vulkan 1.3.
    erase_if(vkDevices, [](VkPhysicalDevice dev) {
        VkPhysicalDeviceProperties props;
        vkFunctions->vkGetPhysicalDeviceProperties(dev, &props);
        return props.apiVersion < VK_API_VERSION_1_3;
    });

    if (vkDevices.isEmpty()) {
        QMessageBox::critical(
            nullptr,
            "Error",
            "No any Vulkan device supports Vulkan 1.3.");
        return 1;
    }

    // Filter out devices that does not support graphics operations.
    erase_if(vkDevices, [](VkPhysicalDevice dev) {
        // Get number of queue family.
        uint32_t count;

        vkFunctions->vkGetPhysicalDeviceQueueFamilyProperties(dev, &count, nullptr);

        // Get queue family.
        QList<VkQueueFamilyProperties> families(count);

        vkFunctions->vkGetPhysicalDeviceQueueFamilyProperties(dev, &count, families.data());

        for (auto &f : families) {
            if (f.queueFlags & VK_QUEUE_GRAPHICS_BIT) {
                return false;
            }
        }

        return true;
    });

    if (vkDevices.isEmpty()) {
        QMessageBox::critical(
            nullptr,
            "Error",
            "No any Vulkan device supports graphics operations.");
        return 1;
    }
#endif

    // Check if no any required settings.
    if (!hasRequiredUserSettings() || !isSystemInitialized()) {
        InitializeWizard init;

        if (!init.exec()) {
            return 1;
        }
    }

    // Setup main window.
#ifdef __APPLE__
    MainWindow win(args);
#else
    MainWindow win(args, &vulkan, std::move(vkDevices));
#endif

    if (!win.loadProfiles() || !win.loadGames()) {
        return 1;
    }

    win.restoreGeometry();

    // Run main window.
    if (args.isSet(Args::debug)) {
        win.startDebug(args.value(Args::debug));
    }

    return QApplication::exec();
}
