#include "pkg_extractor.hpp"

PkgExtractor::PkgExtractor(Pkg &&pkg, std::string &&dst) :
    m_pkg(std::move(pkg)),
    m_dst(std::move(dst))
{
}

PkgExtractor::~PkgExtractor()
{
}

void PkgExtractor::exec()
{
    Error e = pkg_extract(
        m_pkg,
        m_dst.c_str(),
        [](const char *status, std::size_t bar, std::uint64_t current, std::uint64_t total, void *ud) {
            reinterpret_cast<PkgExtractor *>(ud)->update(status, bar, current, total);
        },
        this);

    if (e) {
        emit finished(e.message());
    } else {
        emit finished(QString());
    }
}

void PkgExtractor::update(const char *status, std::size_t bar, std::uint64_t current, std::uint64_t total)
{
    emit statusChanged(status ? QString(status) : QString(), bar, current, total);
}
