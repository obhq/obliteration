#pragma once

struct context;

extern "C" {
    context *context_new(char **error);
    void context_free(context *ctx);
}
