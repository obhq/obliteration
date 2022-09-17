#include "game_models.hpp"

Game::Game(const QString &directory) :
    m_directory(directory)
{
}

Game::~Game()
{
}

GameListModel::GameListModel()
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
