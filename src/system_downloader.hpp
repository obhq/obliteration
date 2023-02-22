#pragma once

#include <QObject>
#include <QString>

#include <cinttypes>

class SystemDownloader : public QObject {
    Q_OBJECT
public:
    SystemDownloader(const QString &from, const QString &to, bool explicitDecryption);
    ~SystemDownloader();

public slots:
    void exec();

signals:
    void statusChanged(const QString &status, std::uint64_t total, std::uint64_t written);
    void finished(const QString &error);

private:
    void update(const char *status, std::uint64_t total, std::uint64_t written);

private:
    QString m_from;
    QString m_to;
    bool m_explicitDecryption;
};
