#pragma once

#include "context.hpp"

struct emulator_config {
};

extern "C" {
    char *emulator_start(context *ctx, const emulator_config *conf);
    int emulator_running(const context *ctx);
}
