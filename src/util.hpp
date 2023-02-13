#pragma once

#include <QString>

// Create a copy of s then invoke std::free on s.
QString fromMalloc(char *s);
