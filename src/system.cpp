#include "system.hpp"
#include "path.hpp"
#include "progress_dialog.hpp"
#include "pup.hpp"
#include "settings.hpp"
#include "string.hpp"

#include <QDir>
#include <QFileDialog>
#include <QMessageBox>

bool hasSystemFilesInstalled()
{
    auto libkernel = toPath(readSystemDirectorySetting());

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

bool updateSystemFiles(QWidget *parent)
{
    // Browse for PS4UPDATE1.PUP.dec.
    auto pupPath = QDir::toNativeSeparators(QFileDialog::getOpenFileName(parent, "Install PS4UPDATE1.PUP.dec", QString(), "PS4UPDATE1.PUP.dec")).toStdString();

    if (pupPath.empty()) {
        return false;
    }

    // Setup progress dialog.
    ProgressDialog progress("Installing PS4UPDATE1.PUP.dec", QString("Opening %1").arg(pupPath.c_str()), parent);

    // Open PS4UPDATE1.PUP.dec.
    Error error;
    Pup pup(pup_open(pupPath.c_str(), &error));

    if (!pup) {
        QMessageBox::critical(&progress, "Error", QString("Failed to open %1: %2").arg(pupPath.c_str()).arg(error.message()));
        return false;
    }

    // Dump system image.
    auto output = joinPath(readSystemDirectorySetting(), "system");
    error = pup_dump_system(pup, output.c_str(), [](const char *name, std::uint64_t total, std::uint64_t written, void *ud) {
        auto toProgress = [total](std::uint64_t v) -> int {
            if (total >= 1024UL*1024UL*1024UL*1024UL) { // >= 1TB
                return v / (1024UL*1024UL*1024UL*10UL); // 10GB step.
            } else if (total >= 1024UL*1024UL*1024UL*100UL) { // >= 100GB
                return v / (1024UL*1024UL*1024UL); // 1GB step.
            } else if (total >= 1024UL*1024UL*1024UL*10UL) { // >= 10GB
                return v / (1024UL*1024UL*100UL); // 100MB step.
            } else if (total >= 1024UL*1024UL*1024UL) { // >= 1GB
                return v / (1024UL*1024UL*10UL); // 10MB step.
            } else if (total >= 1024UL*1024UL*100UL) { // >= 100MB
                return v / (1024UL*1024UL);// 1MB step.
            } else {
                return v;
            }
        };

        auto progress = reinterpret_cast<ProgressDialog *>(ud);
        auto max = toProgress(total);
        auto value = toProgress(written);
        auto label = QString("Installing %1...").arg(name);

        if (progress->statusText() != label) {
            progress->setStatusText(label);
            progress->setValue(0);
            progress->setMaximum(max);
        } else {
            progress->setValue(value == max && written != total ? value - 1 : value);
        }
    }, &progress);

    progress.complete();

    if (error) {
        QMessageBox::critical(parent, "Error", QString("Failed to install %1 to %2: %3").arg(pupPath.c_str()).arg(output.c_str()).arg(error.message()));
        return false;
    }

    QMessageBox::information(parent, "Success", "Installation completed successfully.");

    return true;
}
