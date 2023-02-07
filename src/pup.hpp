#pragma once

#include "error.hpp"

struct pup;

typedef void (*pup_dump_status_t) (const char *name, std::uint64_t total, std::uint64_t written, void *ud);

extern "C" {
    pup *pup_open(const char *file, error **err);
    error *pup_dump_system(const pup *pup, const char *path, pup_dump_status_t status, void *ud);
    void pup_free(pup *pup);
}

class Pup final {
public:
    Pup(pup *obj): m_obj(obj) {}
    Pup(const Pup &) = delete;
    ~Pup()
    {
        if (m_obj) {
            pup_free(m_obj);
        }
    }

public:
    Pup &operator=(const Pup &) = delete;

    operator const pup *() const { return m_obj; }
    operator bool() const { return m_obj != nullptr; }

private:
    pup *m_obj;
};
