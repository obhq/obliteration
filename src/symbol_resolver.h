#ifndef SYMBOLRESOLVER_H
#define SYMBOLRESOLVER_H

#include <optional>
#include <stdint.h>
#include <string>
#include <vector>

#include <QDir>
#include <QMap>
#include <QString>

typedef QString nid;

class SymbolResolver
{
    QMap<QString, std::vector<std::pair<uint64_t, nid>>> symbols_cache;
    QMap<nid, QString> nid_cache;

    QDir orbisPs4ToolchainDir;
    QDir ps4LibDocPath;

    QDir ps4systemDir;
    QDir gameDir;

    std::optional<QString> locate_lib(const QString& searchPath, const QString& library, bool searchElf = true);
    std::vector<std::pair<uint64_t, nid>> read_nids(const QString& library);
    QString resolve_nid(const QString& library, const nid& nid);

public:
    SymbolResolver(QDir orbisPs4ToolchainDir, QDir ps4LibDocPath, QDir ps4systemDir);
    std::optional<std::pair<QString, uint64_t>> resolve(const QString &library, uint64_t offset);
    void setGameDir(const QString& gameDir);

    static std::string demangle(const std::string& symbol);
};

#endif // SYMBOLRESOLVER_H
