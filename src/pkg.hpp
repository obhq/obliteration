#pragma once

#include "context.hpp"
#include "error.hpp"

#include <cinttypes>
#include <cstddef>

struct pkg;
struct pkg_param;

typedef void (*pkg_dump_pfs_status_t) (std::uint64_t written, std::uint64_t total, const char *name, void *ud);

#define PKG_ENTRY_PARAM_SFO 0x00001000 // param.sfo
#define PKG_ENTRY_PIC1_PNG  0x00001006 // pic1.png
#define PKG_ENTRY_ICON0_PNG 0x00001200 // icon0.png

extern "C" {
    // The returned pkg must not outlive ctx.
    pkg *pkg_open(const context *ctx, const char *file, char **error);
    void pkg_close(pkg *pkg);

    pkg_param *pkg_get_param(const pkg *pkg, char **error);
    char *pkg_dump_entry(const pkg *pkg, std::uint32_t id, const char *file);

    // Dump all files from outer PFS.
    error *pkg_dump_pfs(const pkg *pkg, const char *dir, pkg_dump_pfs_status_t status, void *ud);

    pkg_param *pkg_param_open(const char *file, char **error);
    char *pkg_param_title_id(const pkg_param *param);
    char *pkg_param_title(const pkg_param *param);
    void pkg_param_close(pkg_param *param);
}
