#pragma once

#include <QWindow>

class Screen final : public QWindow {
    Q_OBJECT
public:
    Screen();
    ~Screen() override;
signals:
    void updateRequestReceived();
protected:
    bool event(QEvent *ev) override;
};
