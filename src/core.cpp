#include "core.hpp"

extern "C" void qstring_set(QString &s, const unsigned char *v, size_t l)
{
    s = QUtf8StringView(v, l).toString();
}
