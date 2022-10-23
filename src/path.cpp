#include "path.hpp"

using std::filesystem::path;

path toPath(const QString &v)
{
#ifdef _WIN32
    return path(v.toStdWString(), path::native_format);
#else
    return path(v.toStdString(), path::native_format);
#endif
}
