#pragma once

#include "core.h"

#include <QWidget>

class DisplaySettings;
class GameListModel;
class ProfileList;
class QComboBox;
class QLayout;
class QTableView;

class LaunchSettings final : public QWidget {
    Q_OBJECT
public:
    LaunchSettings(ProfileList *profiles, GameListModel *games, QWidget *parent = nullptr);
    ~LaunchSettings() override;
signals:
    void saveClicked(Profile *p);
    void startClicked();
private:
    QWidget *buildSettings(GameListModel *games);
    QLayout *buildActions(ProfileList *profiles);

    void requestGamesContextMenu(const QPoint &pos);
    void profileChanged(int index);

    DisplaySettings *m_display;
    QTableView *m_games;
    QComboBox *m_profiles;
};
