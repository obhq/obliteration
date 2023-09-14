#pragma once

#include <QAbstractListModel>
#include <QList>
#include <QPixmap>

class Game final : public QObject {
public:
    Game(const QString &name, const QString &directory);
    ~Game() override;

public:
    const QString &name() const { return m_name; }
    const QString &directory() const { return m_directory; }
    QPixmap icon() const;

private:
    QString m_name;
    QString m_directory;
};

class GameListModel final : public QAbstractListModel {
public:
    GameListModel(QObject *parent = nullptr);
    ~GameListModel();

public:
    void add(Game *game);
    Game *get(int i) const { return m_items[i]; }
    void clear();
    void sortNames();

public:
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;

private:
    QList<Game *> m_items;
};
