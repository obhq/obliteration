#pragma once

#include <QString>

#include <filesystem>
#include <string>

// Do not throw. Return empty string is case of error.
std::string joinPath(const QString &base, const QString &name);
std::string joinPathStr(const std::string &base, const std::string &name);
// v must be native format.
std::filesystem::path toPath(const QString &v);
