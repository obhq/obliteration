#pragma once

#include "core.hpp"

#include <QAbstractListModel>

#include <vector>

class ProfileList final : public QAbstractListModel {
public:
    ProfileList(QObject *parent = nullptr);
    ~ProfileList() override;

    void add(Rust<Profile> &&p);
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    Profile *get(size_t i) const { return m_items[i]; }
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    void sort(int column, Qt::SortOrder order = Qt::AscendingOrder) override;
private:
    std::vector<Rust<Profile>> m_items;
};
