#pragma once

#include <QWidget>

class DisplaySettings;
class GameListModel;
class QComboBox;
class QLayout;
class QTableView;

class LaunchSettings final : public QWidget {
    Q_OBJECT
public:
    LaunchSettings(GameListModel *games, QWidget *parent = nullptr);
    ~LaunchSettings() override;
signals:
    void startClicked();
private:
    QWidget *buildSettings(GameListModel *games);
    QLayout *buildActions();

    void requestGamesContextMenu(const QPoint &pos);

    DisplaySettings *m_display;
    QTableView *m_games;
    QComboBox *m_profiles;
};
