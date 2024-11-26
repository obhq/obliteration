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
 * Display resolution to report to the kernel.
 */
enum DisplayResolution {
    /**
     * 1280 × 720.
     */
    DisplayResolution_Hd,
    /**
     * 1920 × 1080.
     */
    DisplayResolution_FullHd,
    /**
     * 3840 × 2160.
     */
    DisplayResolution_UltraHd,
};

/**
 * Contains settings to launch the kernel.
 */
struct Profile;

/**
 * Error object managed by Rust side.
 */
struct RustError;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

void error_free(struct RustError *e);

const char *error_message(const struct RustError *e);

struct Profile *profile_new(const char *name);

struct Profile *profile_load(const char *path, struct RustError **err);

void profile_free(struct Profile *p);

char *profile_id(const struct Profile *p);

const char *profile_name(const struct Profile *p);

enum DisplayResolution profile_display_resolution(const struct Profile *p);

void profile_set_display_resolution(struct Profile *p, enum DisplayResolution v);

struct RustError *profile_save(const struct Profile *p, const char *path);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
