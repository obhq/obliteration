#pragma once

#include <QWidget>

class GameSettings;
class QComboBox;
class QGroupBox;

class GameGraphicSettings final : public QWidget {
public:
    GameGraphicSettings(GameSettings *settings, QWidget *parent = nullptr);
    ~GameGraphicSettings();

private:
    QGroupBox *setupModeWidget(GameSettings *settings);

private:
    QComboBox *m_mode;
};
