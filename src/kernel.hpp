#pragma once

#include "error.hpp"

struct kernel;

extern "C" {
    kernel *kernel_new(error **err);
    void kernel_shutdown(kernel *krn);
}
