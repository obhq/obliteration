#include "profile_models.hpp"

ProfileList::ProfileList(QObject *parent) :
    QAbstractListModel(parent)
{
}

ProfileList::~ProfileList()
{
}

void ProfileList::add(Rust<Profile> &&p)
{
    beginInsertRows(QModelIndex(), m_items.size(), m_items.size());
    m_items.push_back(std::move(p));
    endInsertRows();

    sort(0);
}

int ProfileList::rowCount(const QModelIndex &) const
{
    return m_items.size();
}

QVariant ProfileList::data(const QModelIndex &index, int role) const
{
    auto &p = m_items[index.row()];

    switch (role) {
    case Qt::DisplayRole:
        return profile_name(p);
    }

    return {};
}

void ProfileList::sort(int column, Qt::SortOrder order)
{
    emit layoutAboutToBeChanged();

    switch (column) {
    case 0:
        std::sort(
            m_items.begin(),
            m_items.end(),
            [order](const Rust<Profile> &a, const Rust<Profile> &b) {
                if (order == Qt::AscendingOrder) {
                    return strcmp(profile_name(a), profile_name(b));
                } else {
                    return strcmp(profile_name(b), profile_name(a));
                }
            });
        break;
    }

    emit layoutChanged();
}
