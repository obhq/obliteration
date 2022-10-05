#pragma once

#include "error.hpp"

struct kernel;
struct kernel_rootfs;
struct kernel_pfs;

typedef void (*kernel_logger_t) (int pid, int err, const char *msg, void *ud);

extern "C" {
    // The returned kernel will take the ownership of rootfs.
    kernel *kernel_new(kernel_rootfs *rootfs, error **err);
    void kernel_shutdown(kernel *krn);

    void kernel_set_logger(kernel *krn, kernel_logger_t logger, void *ud);

    kernel_rootfs *kernel_rootfs_new(error **err);
    void kernel_rootfs_free(kernel_rootfs *fs);
}
