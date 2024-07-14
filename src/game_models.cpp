#include "game_models.hpp"
#include "path.hpp"

#include <QFile>

Game::Game(const QString &id, const QString &name, const QString &directory) :
    m_id(id),
    m_name(name),
    m_directory(directory)
{
    // Load icon.
    auto dir = joinPath(directory, "sce_sys");
    auto path = joinPath(dir.c_str(), "icon0.png");
    QPixmap icon(QFile::exists(path.c_str()) ? path.c_str() : ":/resources/fallbackicon0.png");

    icon.setDevicePixelRatio(2.0);

    // Scale down.
    m_icon = icon.scaled(64, 64, Qt::KeepAspectRatio, Qt::SmoothTransformation);
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

    beginInsertRows(QModelIndex(), m_items.size(), m_items.size());
    m_items.append(game);
    endInsertRows();

    sort(0);
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

int GameListModel::columnCount(const QModelIndex &) const
{
    return 2;
}

int GameListModel::rowCount(const QModelIndex &) const
{
    return m_items.size();
}

QVariant GameListModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (role != Qt::DisplayRole) {
        return {};
    } else if (orientation == Qt::Vertical) {
        return section + 1;
    } else if (orientation != Qt::Horizontal) {
        return {};
    }

    switch (section) {
    case 0:
        return "Name";
    case 1:
        return "ID";
    default:
        return {};
    }
}

QVariant GameListModel::data(const QModelIndex &index, int role) const
{
    auto game = m_items[index.row()];

    switch (index.column()) {
    case 0:
        switch (role) {
        case Qt::DisplayRole:
            return game->name();
        case Qt::DecorationRole:
            return game->icon();
        }
        break;
    case 1:
        if (role == Qt::DisplayRole) {
            return game->id();
        }
        break;
    }

    return {};
}

void GameListModel::sort(int column, Qt::SortOrder order)
{
    emit layoutAboutToBeChanged();

    switch (column) {
    case 0:
        std::sort(m_items.begin(), m_items.end(), [order](const Game *a, const Game *b) {
            if (order == Qt::AscendingOrder) {
                return a->name().toUpper() < b->name().toUpper();
            } else {
                return a->name().toUpper() > b->name().toUpper();
            }
        });
        break;
    case 1:
        std::sort(m_items.begin(), m_items.end(), [order](const Game *a, const Game *b) {
            if (order == Qt::AscendingOrder) {
                return a->id() < b->id();
            } else {
                return a->id() > b->id();
            }
        });
        break;
    }

    emit layoutChanged();
}
