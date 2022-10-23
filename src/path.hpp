#pragma once

#include <QString>

#include <filesystem>

// v must be native format.
std::filesystem::path toPath(const QString &v);
