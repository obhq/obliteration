#include "util.hpp"

#include <filesystem>

std::string joinPath(const QString &base, const QString &name)
{
    std::string r;

    try {
#ifdef _WIN32
        std::filesystem::path p(base.toStdWString(), std::filesystem::path::native_format);
        p /= name.toStdWString();
#else
        std::filesystem::path p(base.toStdString(), std::filesystem::path::native_format);
        p /= name.toStdString();
#endif
        r = p.u8string();
    } catch (...) {
        return std::string();
    }

    return r;
}
