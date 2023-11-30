#include "path.hpp"

using std::filesystem::path;

std::string joinPath(const QString &base, const QString &name)
{
    try {
        auto p = toPath(base);
#ifdef _WIN32
        p /= name.toStdWString();
#else
        p /= name.toStdString();
#endif
        return p.u8string();
    } catch (...) {
        return std::string();
    }
}

// For doing joins to a preexisting path.
std::string joinPathStr(const std::string &base, const std::string &name) {
    QString qBase = QString::fromStdString(base);
    QString qName = QString::fromStdString(name);
    return joinPath(qBase, qName);
}

path toPath(const QString &v)
{
#ifdef _WIN32
    return path(v.toStdWString(), path::native_format);
#else
    return path(v.toStdString(), path::native_format);
#endif
}
