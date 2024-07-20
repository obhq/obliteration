#pragma once

#include <QString>

class QWidget;

bool isSystemInitialized();
bool isSystemInitialized(const QString &path);
bool initSystem(const QString &path, const QString &firmware, QWidget *parent = nullptr);
