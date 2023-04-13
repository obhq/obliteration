#pragma once

#include <QString>

class QWidget;

bool isSystemInitialized();
bool isSystemInitialized(const QString &path);
bool initSystem(const QString &path, const QString &from, bool explicitDecryption, QWidget *parent = nullptr);
bool ensureSystemDirectories(QWidget *parent = nullptr);
