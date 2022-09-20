#pragma once

#include <QString>

#include <string>

// Do not throw. Return empty string is case of error.
std::string joinPath(const QString &base, const QString &name);
