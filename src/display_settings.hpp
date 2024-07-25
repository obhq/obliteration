#pragma once

#include <QWidget>

class DisplaySettings final : public QWidget {
public:
    DisplaySettings(QWidget *parent = nullptr);
    ~DisplaySettings() override;
};
