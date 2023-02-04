#pragma once

#include "error.hpp"

#include <cinttypes>
#include <cstddef>

struct pkg;
struct pkg_param;

typedef void (*pkg_extract_status_t) (const char *name, std::uint64_t total, std::uint64_t written, void *ud);

extern "C" {
    pkg *pkg_open(const char *file, error **error);
    void pkg_close(pkg *pkg);

    pkg_param *pkg_get_param(const pkg *pkg, char **error);
    error *pkg_extract(const pkg *pkg, const char *dir, pkg_extract_status_t status, void *ud);

    pkg_param *pkg_param_open(const char *file, char **error);
    char *pkg_param_title_id(const pkg_param *param);
    char *pkg_param_title(const pkg_param *param);
    void pkg_param_close(pkg_param *param);
}

class Pkg final {
public:
    Pkg() : m_obj(nullptr) {}
    Pkg(const Pkg &) = delete;
    ~Pkg() { close(); }

public:
    Pkg &operator=(const Pkg &) = delete;
    Pkg &operator=(pkg *obj)
    {
        if (m_obj) {
            pkg_close(m_obj);
        }

        m_obj = obj;
        return *this;
    }

    operator pkg *() const { return m_obj; }

public:
    void close()
    {
        if (m_obj) {
            pkg_close(m_obj);
            m_obj = nullptr;
        }
    }

private:
    pkg *m_obj;
};
