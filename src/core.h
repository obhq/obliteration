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

/**
 * Manage a virtual machine that run the kernel.
 */
struct Vmm;

/**
 * Contains objects required to render the screen.
 */
struct VmmScreen {
#if !defined(__APPLE__)
    size_t vk_instance
#endif
    ;
#if !defined(__APPLE__)
    size_t vk_surface
#endif
    ;
#if defined(__APPLE__)
    size_t view
#endif
    ;
};

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

void error_free(struct RustError *e);

const char *error_message(const struct RustError *e);

Param *param_open(const char *file, struct RustError **error);

void param_close(Param *p);

char *param_app_ver_get(const Param *p);

char *param_category_get(const Param *p);

char *param_content_id_get(const Param *p);

char *param_short_content_id_get(const Param *p);

char *param_title_get(const Param *p);

char *param_title_id_get(const Param *p);

char *param_version_get(const Param *p);

Pkg *pkg_open(const char *file, struct RustError **error);

void pkg_close(Pkg *pkg);

Param *pkg_get_param(const Pkg *pkg, struct RustError **error);

struct RustError *pkg_extract(const Pkg *pkg, const char *dir, void (*status)(const char*,
                                                                              size_t,
                                                                              uint64_t,
                                                                              uint64_t,
                                                                              void*), void *ud);

struct RustError *update_firmware(const char *root,
                                  const char *fw,
                                  void *cx,
                                  void (*status)(const char*, uint64_t, uint64_t, void*));

void vmm_free(struct Vmm *vmm);

struct Vmm *vmm_run(const char *kernel, const struct VmmScreen *screen, struct RustError **err);

struct RustError *vmm_draw(struct Vmm *vmm);

void vmm_logs(const struct Vmm *vmm, void *cx, void (*cb)(uint8_t, const char*, size_t, void*));

#if defined(__linux__)
extern int kvm_check_version(int kvm, bool *compat);
#endif

#if defined(__linux__)
extern int kvm_max_vcpus(int kvm, size_t *max);
#endif

#if defined(__linux__)
extern int kvm_create_vm(int kvm, int *fd);
#endif

#if defined(__linux__)
extern int kvm_get_vcpu_mmap_size(int kvm);
#endif

#if defined(__linux__)
extern int kvm_set_user_memory_region(int vm,
                                      uint32_t slot,
                                      uint64_t addr,
                                      uint64_t len,
                                      void *mem);
#endif

#if defined(__linux__)
extern int kvm_create_vcpu(int vm, uint32_t id, int *fd);
#endif

#if defined(__linux__)
extern int kvm_run(int vcpu);
#endif

#if defined(__linux__)
extern int kvm_get_regs(int vcpu, kvm_regs *regs);
#endif

#if defined(__linux__)
extern int kvm_set_regs(int vcpu, const kvm_regs *regs);
#endif

#if defined(__linux__)
extern int kvm_get_sregs(int vcpu, kvm_sregs *regs);
#endif

#if defined(__linux__)
extern int kvm_set_sregs(int vcpu, const kvm_sregs *regs);
#endif

#if defined(__linux__)
extern int kvm_translate(int vcpu, kvm_translation *arg);
#endif

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus
