#pragma once

#include <QAbstractListModel>
#include <QList>

class Game final : public QObject {
public:
    Game(const QString &name, const QString &file);
    ~Game() override;

public:
    const QString &name() const { return m_name; }
    const QString &file() const { return m_file; }

private:
    QString m_name;
    QString m_file;
};

class GameListModel final : public QAbstractListModel {
public:
    GameListModel(QObject *parent = nullptr);
    ~GameListModel();

public:
    void add(Game *game);
    Game *get(int i) const { return m_items[i]; }
    void clear();

public:
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;

private:
    QList<Game *> m_items;
};
