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
    auto output = readSystemDirectorySetting().toStdString();
    error = pup_dump_system(pup, output.c_str());

    if (error) {
        QMessageBox::critical(&progress, "Error", QString("Failed to install %1 to %2: %3").arg(pupPath.c_str()).arg(output.c_str()).arg(error.message()));
        return false;
    }

    return true;
}
