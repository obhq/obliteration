#include "system.hpp"
#include "core.hpp"
#include "path.hpp"
#include "progress_dialog.hpp"
#include "settings.hpp"

#include <QMessageBox>

bool isSystemInitialized()
{
    return isSystemInitialized(readSystemDirectorySetting());
}

bool isSystemInitialized(const QString &path)
{
    auto root = toPath(path);
    std::filesystem::file_status status;

    try {
#ifdef _WIN32
        status = std::filesystem::status(root / L"part" / L"md0.obp");
#else
        status = std::filesystem::status(root / "part" / "md0.obp");
#endif
    } catch (...) {
        return false;
    }

    return status.type() == std::filesystem::file_type::regular;
}

bool initSystem(const QString &path, const QString &firmware, QWidget *parent)
{
    // Setup progress dialog.
    ProgressDialog progress("Initializing system", QString("Opening %1").arg(firmware), parent);

    // Update firmware.
    auto root = path.toStdString();
    auto fw = firmware.toStdString();
    RustPtr<RustError> error;

    error = update_firmware(
        root.c_str(),
        fw.c_str(),
        &progress, [](const char *status, std::uint64_t total, std::uint64_t written, void *cx) {
            auto progress = reinterpret_cast<ProgressDialog *>(cx);

            if (progress->statusText() != status) {
                progress->setStatusText(status);
                progress->setValue(0);
                progress->setMaximum(total);
            } else {
                progress->setValue(written);
            }
        });

    progress.complete();

    // Check result.
    if (error) {
        QMessageBox::critical(
            parent,
            "Error",
            QString("Failed to install %1 to %2: %3").arg(firmware).arg(path).arg(error_message(error)));
        return false;
    }

    QMessageBox::information(parent, "Success", "Firmware installed successfully.");

    return true;
}
