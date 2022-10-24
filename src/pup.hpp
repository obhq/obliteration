#pragma once

#include "error.hpp"

struct pup;

extern "C" {
    pup *pup_open(const char *file, error **err);
    error *pup_dump_system(const pup *pup, const char *path);
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
