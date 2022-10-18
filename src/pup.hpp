#pragma once

#include "error.hpp"

struct pup;

extern "C" {
    pup *pup_open(const char *file, error **err);
    void pup_free(pup *pup);
}
