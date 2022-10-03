#pragma once

#include "error.hpp"

struct kernel;

typedef void (*kernel_logger_t) (int pid, int err, const char *msg, void *ud);

extern "C" {
    kernel *kernel_new(error **err);
    void kernel_shutdown(kernel *krn);

    void kernel_set_logger(kernel *krn, kernel_logger_t logger, void *ud);
}
