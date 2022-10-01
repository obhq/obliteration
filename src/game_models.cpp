#include "game_models.hpp"
#include "pkg.hpp"
#include "util.hpp"

Game::Game(const QString &name, const QString &directory) :
    m_name(name),
    m_directory(directory)
{
}

Game::~Game()
{
}

QPixmap Game::icon() const
{
    QPixmap icon(joinPath(directory(), PKG_ENTRY_ICON0_PNG).c_str());

    icon.setDevicePixelRatio(2.0);

    return icon;
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

    beginInsertRows(QModelIndex(), m_items.size(), m_items.size());
    m_items.append(game);
    endInsertRows();
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
        return m_items[index.row()]->name();
    case Qt::DecorationRole:
        return m_items[index.row()]->icon();
    default:
        return QVariant();
    }
}
