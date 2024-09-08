#pragma once

#include <QWidget>

class CpuSettings final : public QWidget {
public:
    CpuSettings(QWidget *parent = nullptr);
    ~CpuSettings() override;
private:
    QWidget *buildCount();
};
