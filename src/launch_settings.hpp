#pragma once

#include <QWidget>

class LaunchSettings final : public QWidget {
    Q_OBJECT
public:
    LaunchSettings(QWidget *parent = nullptr);
    ~LaunchSettings() override;

signals:
    void startClicked();
};
