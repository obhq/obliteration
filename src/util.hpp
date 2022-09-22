#pragma once

#include <QString>

#include <string>

// Do not throw. Return empty string is case of error.
std::string joinPath(const QString &base, const QString &name);

// Create a copy of s then invoke std::free on s.
QString fromMalloc(char *s);
