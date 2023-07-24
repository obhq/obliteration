#pragma once

#include <QString>
#include <QWidget>

class QCheckBox;
class QGroupBox;
class QLineEdit;

class UpdateFirmware : public QWidget {
public:
    UpdateFirmware(QWidget *parent = nullptr);
    ~UpdateFirmware();

public:
    QString from() const;
    bool explicitDecryption() const;

private:
    QLineEdit *m_from;
    QCheckBox *m_explicitDecryption;
};
