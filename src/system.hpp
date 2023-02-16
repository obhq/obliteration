#pragma once

#include <QString>

class QWidget;

bool hasSystemFilesInstalled();
bool hasSystemFilesInstalled(const QString &systemPath);
bool updateSystemFiles(QWidget *parent = nullptr);
bool updateSystemFiles(const QString &systemPath, QWidget *parent = nullptr);
