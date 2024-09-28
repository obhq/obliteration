// This file contains KVM wrappers for Rust side. The reason we need these wrappers is because the
// KVM ioctls is not available in the libc binding.
#include "core.h"

#include <sys/ioctl.h>

#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

extern "C" int kvm_set_user_memory_region(
    int vm,
    uint32_t slot,
    uint64_t addr,
    uint64_t len,
    void *mem)
{
    kvm_userspace_memory_region mr;

    memset(&mr, 0, sizeof(mr));

    mr.slot = slot;
    mr.guest_phys_addr = addr;
    mr.memory_size = len;
    mr.userspace_addr = reinterpret_cast<uint64_t>(mem);

    if (ioctl(vm, KVM_SET_USER_MEMORY_REGION, &mr) < 0) {
        return errno;
    }

    return 0;
}

extern "C" int kvm_run(int vcpu)
{
    return ioctl(vcpu, KVM_RUN, 0);
}

#ifndef __aarch64__
extern "C" int kvm_get_regs(int vcpu, kvm_regs *regs)
{
    return ioctl(vcpu, KVM_GET_REGS, regs);
}

extern "C" int kvm_set_regs(int vcpu, const kvm_regs *regs)
{
    return ioctl(vcpu, KVM_SET_REGS, regs);
}
#endif

#ifdef __x86_64__
extern "C" int kvm_get_sregs(int vcpu, kvm_sregs *regs)
{
    return ioctl(vcpu, KVM_GET_SREGS, regs);
}

extern "C" int kvm_set_sregs(int vcpu, const kvm_sregs *regs)
{
    return ioctl(vcpu, KVM_SET_SREGS, regs);
}

extern "C" int kvm_translate(int vcpu, kvm_translation *arg) {
    return ioctl(vcpu, KVM_TRANSLATE, arg);
}
#endif
