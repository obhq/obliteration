#pragma once

#include "context.hpp"

#include <cinttypes>
#include <cstddef>

struct pkg;
struct pkg_entry;

#define PKG_ENTRY_PARAM_SFO 0x00001000 // param.sfo
#define PKG_ENTRY_PIC1_PNG  0x00001006 // pic1.png
#define PKG_ENTRY_ICON0_PNG 0x00001200 // icon0.png

extern "C" {
    // The returned pkg must not outlive ctx.
    pkg *pkg_open(const context *ctx, const char *file, char **error);
    void pkg_close(pkg *pkg);

    void *pkg_enum_entries(const pkg *pkg, void * (*cb) (const pkg_entry *, std::size_t, void *), void *ud);

    std::uint32_t pkg_entry_id(const pkg_entry *entry);
    char *pkg_entry_dump(const pkg_entry *entry, const char *file);
}
