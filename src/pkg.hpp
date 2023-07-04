#pragma once

#include "error.hpp"
#include "param.hpp"

#include <cinttypes>
#include <cstddef>

struct pkg;

typedef void (*pkg_extract_status_t) (const char *name, std::uint64_t total, std::uint64_t written, void *ud);

extern "C" {
    pkg *pkg_open(const char *file, error **error);
    void pkg_close(pkg *pkg);

    param *pkg_get_param(const pkg *pkg, error **error);
    error *pkg_extract(const pkg *pkg, const char *dir, pkg_extract_status_t status, void *ud);
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
