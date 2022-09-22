#include "util.hpp"

#include <cstdlib>
#include <filesystem>

std::string joinPath(const QString &base, const QString &name)
{
    try {
#ifdef _WIN32
        std::filesystem::path p(base.toStdWString(), std::filesystem::path::native_format);
        p /= name.toStdWString();
#else
        std::filesystem::path p(base.toStdString(), std::filesystem::path::native_format);
        p /= name.toStdString();
#endif
        return p.u8string();
    } catch (...) {
        return std::string();
    }
}

QString fromMalloc(char *s)
{
    QString r(s);
    std::free(s);
    return r;
}
