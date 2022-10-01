#pragma once

#include "context.hpp"
#include "error.hpp"

#include <cinttypes>
#include <cstddef>

struct pkg;
struct pkg_param;

typedef void (*pkg_dump_pfs_status_t) (std::uint64_t written, std::uint64_t total, const char *name, void *ud);

#define PKG_ENTRY_PARAM_SFO "entry_4096" // param.sfo
#define PKG_ENTRY_PIC1_PNG  "entry_4102" // pic1.png
#define PKG_ENTRY_ICON0_PNG "entry_4608" // icon0.png

extern "C" {
    // The returned pkg must not outlive ctx.
    pkg *pkg_open(const context *ctx, const char *file, char **error);
    void pkg_close(pkg *pkg);

    pkg_param *pkg_get_param(const pkg *pkg, char **error);
    error *pkg_dump_entries(const pkg *pkg, const char *dir);

    // Dump all files from outer PFS.
    error *pkg_dump_pfs(const pkg *pkg, const char *dir, pkg_dump_pfs_status_t status, void *ud);

    pkg_param *pkg_param_open(const char *file, char **error);
    char *pkg_param_title_id(const pkg_param *param);
    char *pkg_param_title(const pkg_param *param);
    void pkg_param_close(pkg_param *param);
}
