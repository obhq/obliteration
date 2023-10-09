#include "system_downloader.hpp"
#include "core.hpp"

extern "C" {
    error *system_download(
        const char *from,
        const char *to,
        bool explicit_decryption,
        void (*status) (const char *, std::uint64_t, std::uint64_t, void *),
        void *ud);
}

SystemDownloader::SystemDownloader(const QString &from, const QString &to, bool explicitDecryption) :
    m_from(from),
    m_to(to),
    m_explicitDecryption(explicitDecryption)
{
}

SystemDownloader::~SystemDownloader()
{
}

void SystemDownloader::exec()
{
    auto from = m_from.toStdString();
    auto to = m_to.toStdString();
    Error error = system_download(from.c_str(), to.c_str(), m_explicitDecryption, [](auto status, auto total, auto written, auto ud) {
        reinterpret_cast<SystemDownloader *>(ud)->update(status, total, written);
    }, this);

    if (error) {
        emit finished(error.message());
    } else {
        emit finished(QString());
    }
}

void SystemDownloader::update(const char *status, std::uint64_t total, std::uint64_t written)
{
    emit statusChanged(status, total, written);
}
