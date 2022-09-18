#include "game_models.hpp"

Game::Game(const QString &directory) :
    m_directory(directory)
{
}

Game::~Game()
{
}

GameListModel::GameListModel(QObject *parent) :
    QAbstractListModel(parent)
{
}

GameListModel::~GameListModel()
{
}

void GameListModel::add(Game *game)
{
    game->setParent(this);
    m_items.append(game);
}

void GameListModel::clear()
{
    beginResetModel();

    for (auto i : m_items) {
        delete i;
    }

    m_items.clear();

    endResetModel();
}

int GameListModel::rowCount(const QModelIndex &) const
{
    return m_items.size();
}

QVariant GameListModel::data(const QModelIndex &index, int role) const
{
    switch (role) {
    case Qt::DisplayRole:
        return m_items[index.row()]->directory();
    default:
        return QVariant();
    }
}
