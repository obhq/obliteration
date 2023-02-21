#pragma once

#include <QString>
#include <QWidget>

class QGroupBox;
class QLineEdit;

class UpdateFirmware : public QWidget {
public:
    UpdateFirmware(QWidget *parent = nullptr);
    ~UpdateFirmware();

public:
    QString from() const;

private:
    QGroupBox *setupFromGroup();

private:
    QLineEdit *m_from;
};
