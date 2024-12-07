#pragma once

#include <QList>
#ifndef __APPLE__
#include <QVulkanInstance>
#endif
#include <QWidget>

class QComboBox;
class QLayout;
class QTableView;

class LaunchSettings final : public QWidget {
    Q_OBJECT
public:
    LaunchSettings(QWidget *parent = nullptr);
    ~LaunchSettings() override;
signals:
    void startClicked(const QString &debugAddr);
private:
    QWidget *buildSettings();
    QLayout *buildActions();

    QComboBox *m_profiles;
};
