#include "util.hpp"

#include <cstdlib>

QString fromMalloc(char *s)
{
    QString r(s);
    std::free(s);
    return r;
}
