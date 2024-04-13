#include "game_models.hpp"
#include "path.hpp"
#include <QFile>

Game::Game(const QString &id, const QString &name, const QString &directory) :
    m_id(id),
    m_name(name),
    m_directory(directory)
{
}

Game::~Game()
{
}

QPixmap Game::icon() const
{
    // Get icon path.
    auto dir = joinPath(m_directory, "sce_sys");
    auto path = joinPath(dir.c_str(), "icon0.png");

    QPixmap icon;

    if (QFile::exists(path.c_str())) {
        icon.load(path.c_str());
    } else {
        // Load fallback icon if icon0 doesn't exist.
        icon.load(":/resources/fallbackicon0.png");
    }

    // For games with large icon sizes.
    if (icon.width() != 512 || icon.height() != 512) {
        icon = icon.scaled(512, 512, Qt::KeepAspectRatio, Qt::SmoothTransformation);
    }

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

void GameListModel::sort(int column, Qt::SortOrder order)
{
    if (column != 0)
        return;

    emit layoutAboutToBeChanged();

    auto compare = [order](const Game* a, const Game* b) {
        if (order == Qt::AscendingOrder)
            return a->name().toLower() < b->name().toLower();
        else
            return a->name().toLower() > b->name().toLower();
    };

    std::sort(m_items.begin(), m_items.end(), compare);

    emit layoutChanged();
}
