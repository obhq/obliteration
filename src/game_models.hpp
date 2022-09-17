#pragma once

#include <QAbstractListModel>
#include <QList>

class Game final : public QObject {
public:
    Game(const QString &directory);
    ~Game();

public:
    const QString &directory() const { return m_directory; }

private:
    QString m_directory;
};

class GameListModel final : public QAbstractListModel {
public:
    GameListModel();
    ~GameListModel();

public:
    void add(Game *game);

public:
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;

private:
    QList<Game *> m_items;
};
