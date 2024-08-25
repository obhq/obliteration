#pragma once

#include "core.h"

#include <QWidget>

class QComboBox;

class DisplaySettings final : public QWidget {
public:
    DisplaySettings(QWidget *parent = nullptr);
    ~DisplaySettings() override;

    void setProfile(Profile *p);
private:
    QWidget *buildResolution();

    QComboBox *m_resolutions;
    Profile *m_profile;
};
