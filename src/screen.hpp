#pragma once

#include <QWindow>

class Screen final : public QWindow {
public:
    Screen();
    ~Screen() override;
};
