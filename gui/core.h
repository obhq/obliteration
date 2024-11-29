#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __linux__
#include <linux/kvm.h>
#endif

struct Param;
struct Pkg;

/**
 * Error object managed by Rust side.
 */
struct RustError;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

void error_free(struct RustError *e);

const char *error_message(const struct RustError *e);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
