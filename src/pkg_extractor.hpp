#pragma once

#include <QObject>

#include "core.hpp"

class PkgExtractor final : public QObject {
    Q_OBJECT
public:
    PkgExtractor(RustPtr<Pkg> &&pkg, std::string &&dst);
    ~PkgExtractor() override;
public slots:
    void exec();
signals:
    void statusChanged(const QString &status, std::size_t bar, std::uint64_t current, std::uint64_t total);
    void finished(const QString &error);
private:
    void update(const char *status, std::size_t bar, std::uint64_t current, std::uint64_t total);
private:
    RustPtr<Pkg> m_pkg;
    std::string m_dst;
};
