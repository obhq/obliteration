#pragma once

#include <QString>

#include <filesystem>
#include <string>

// Do not throw. Return empty string is case of error.
std::string joinPath(const QString &base, const QString &name);

// v must be native format.
std::filesystem::path toPath(const QString &v);
