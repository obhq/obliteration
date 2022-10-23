#include "system.hpp"
#include "path.hpp"
#include "settings.hpp"
#include "string.hpp"

#include <QFileDialog>

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

    return true;
}
