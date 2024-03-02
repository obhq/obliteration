#include "symbol_resolver.h"

#include <QDirIterator>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QRegularExpression>
#include <QString>

SymbolResolver::SymbolResolver(QDir orbisPs4ToolchainDir, QDir ps4LibDocPath, QDir ps4systemDir) :
    orbisPs4ToolchainDir(orbisPs4ToolchainDir), ps4LibDocPath(ps4LibDocPath), ps4systemDir(ps4systemDir)
{
    auto ps4Toolchain = qEnvironmentVariable("OO_PS4_TOOLCHAIN");
    if (!ps4Toolchain.isNull())
        this->orbisPs4ToolchainDir = QDir(ps4Toolchain);

    auto ps4LibDoc = qEnvironmentVariable("PS4LIBDOC");
    if (!ps4LibDoc.isNull())
        this->ps4LibDocPath = QDir(ps4LibDoc);
}

void SymbolResolver::setGameDir(const QString &gameDir)
{
    this->gameDir = QDir(gameDir);
}

std::optional<QString> SymbolResolver::locate_lib(const QString& searchPath, const QString &library, bool searchElf)
{
    auto searchDir = searchPath;
    if (searchElf && (library.startsWith("libSceFios2")
                      || library.startsWith("libc")))
    //  || library.startsWith("") TODO: other libraries provided with the app?
    {
        searchDir = this->gameDir.absoluteFilePath("sce_module");
    }

    QDirIterator it(searchDir, QDir::Files, QDirIterator::Subdirectories);

    while (it.hasNext())
    {
        auto file = it.nextFileInfo();

        if (file.fileName().compare(library) == 0)
            return {file.absoluteFilePath()};
    }

    return {};
}

std::vector<std::pair<uint64_t, nid>> SymbolResolver::read_nids(const QString& library)
{
    auto cached = this->symbols_cache.find(library);
    if (cached != std::end(this->symbols_cache))
        return cached.value();

    auto path = locate_lib(ps4systemDir.absoluteFilePath("system"), library);
    if (!path)
        return {};

    auto readoelf = this->orbisPs4ToolchainDir.absoluteFilePath("bin/linux/readoelf");
    QProcess p;
    p.start(readoelf, {"-s", *path});

    const static auto whitespace = QRegularExpression("\\s+");

    std::vector<std::pair<uint64_t, nid>> ret;

    if (p.waitForFinished(1500) && p.exitCode() == 0)
    {
        p.readLine(); // Symbol table...
        p.readLine(); // column names

        QString line;
        bool ok;

        while (line = p.readLine(), !line.isNull())
        {
            auto info = line.split(whitespace, Qt::SkipEmptyParts);

            if (info.isEmpty())
                continue;

            auto offset = info.at(1).toULongLong(&ok, 16);
            if (offset == 0 || !ok)
                continue;

            auto name = info.at(7);

            ret.emplace_back(offset, name);
        }

        std::sort(std::begin(ret), std::end(ret), [](const std::pair<uint64_t, nid>& p1, const std::pair<uint64_t, nid>& p2){
            return p1.first < p2.first;
        });
        this->symbols_cache.insert(library, ret);

        return ret;
    }
    else
    {
        qDebug() << readoelf << *path << p.exitCode();
        qDebug() << "reading nids from" << library << "failed";
        return {};
    }
}

QString SymbolResolver::resolve_nid(const QString& library, const nid& nid)
{
    auto cached = this->nid_cache.find(nid);
    if (cached != std::end(this->nid_cache))
        return cached.value();

    auto sprxLibrary = QString(library).replace(QString(".prx"), QString(".sprx"));
    auto jsonPath = locate_lib(ps4LibDocPath.absoluteFilePath("system"), sprxLibrary + ".json", false);
    if (!jsonPath)
        return nid;

    QFile file(*jsonPath);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text))
        return nid;

    auto doc = QJsonDocument::fromJson(file.readAll());
    if (doc.isNull())
        return nid;

    auto modules = doc.object().value("modules").toArray();
    for (auto &&module : modules)
    {
        auto object = module.toObject();
        if (library.startsWith(object.value("name").toString()))
        {
            auto libraries = object.value("libraries").toArray();
            for (auto &&library : libraries)
            {
                auto object = library.toObject();
                if (!object.value("is_export").toBool(false))
                    continue;

                for (auto &&symbol : object.value("symbols").toArray())
                {
                    auto object = symbol.toObject();

                    if (nid.startsWith(object.value("encoded_id").toString()))
                    {
                        auto name = object.value("name").toString();
                        this->nid_cache.insert(nid, name);

                        return name;
                    }
                }
            }
        }
    }

    return nid;
}

std::optional<std::pair<QString, uint64_t>> SymbolResolver::resolve(const QString &library, uint64_t offset)
{
    auto symbols = read_nids(library);
    auto last = std::cend(symbols);

    auto subsequentSymbol = std::lower_bound(std::cbegin(symbols), last, offset, [](const std::pair<uint64_t, nid>& p1, const uint64_t& p2){
        return p1.first < p2;
    });

    if (subsequentSymbol == last || subsequentSymbol == std::cbegin(symbols))
        return {};

    auto funNid = std::prev(subsequentSymbol);
    auto funName = resolve_nid(library, (*funNid).second);

    return {{funName, offset - (*funNid).first}};
}

std::string SymbolResolver::demangle(const std::string &symbol)
{
    QProcess p;
    p.start(QString("c++filt"), {QString::fromStdString(symbol)});

    if (p.waitForFinished(1000) && p.exitCode() == 0)
    {
        return p.readLine().trimmed().toStdString();
    }

    return symbol;
}
