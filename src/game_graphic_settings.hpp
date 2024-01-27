#pragma once

#include <QWidget>

class QComboBox;
class QGroupBox;

class GameGraphicSettings final : public QWidget {
public:
    GameGraphicSettings(QWidget *parent = nullptr);
    ~GameGraphicSettings();

private:
    QGroupBox *setupModeWidget();

private:
    QComboBox *m_mode;
};
