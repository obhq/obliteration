#pragma once

#include <QWidget>

class QLineEdit;

class CpuSettings final : public QWidget {
    Q_OBJECT
public:
    CpuSettings(QWidget *parent = nullptr);
    ~CpuSettings() override;
signals:
    void debugClicked(const QString &addr);
private:
    QWidget *buildCount();
    QWidget *buildDebug();

    QLineEdit *m_debugAddr;
};
