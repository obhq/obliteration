#include "system.hpp"
#include "path.hpp"
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
