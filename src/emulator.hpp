// This file is an interface to emulator/src/lib.rs.
#pragma once

#include <cinttypes>
#include <cstddef>

typedef struct context *context_t;
typedef struct pkg *pkg_t;
typedef struct pkg_entry *pkg_entry_t;

struct emulator_config {
};

#define PKG_ENTRY_PARAM_SFO 0x00001000 // param.sfo
#define PKG_ENTRY_PIC1_PNG  0x00001006 // pic1.png
#define PKG_ENTRY_ICON0_PNG 0x00001200 // icon0.png

extern "C" {
    context_t emulator_init(char **error);
    void emulator_term(context_t ctx);

    char *emulator_start(context_t ctx, const emulator_config *conf);
    int emulator_running(context_t ctx);

    // The returned pkg_t must not outlive ctx.
    pkg_t pkg_open(context_t ctx, const char *file, char **error);

    void *pkg_enum_entries(pkg_t pkg, void * (*cb) (pkg_entry_t, std::size_t, void *), void *ctx);
    void pkg_close(pkg_t pkg);

    std::uint32_t pkg_entry_id(pkg_entry_t e);
    char *pkg_entry_read(pkg_entry_t e, const char *file);
}
