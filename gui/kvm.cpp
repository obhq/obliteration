// This file contains KVM wrappers for Rust side. The reason we need these wrappers is because the
// KVM ioctls is not available in the libc binding.
#include "core.h"

#include <sys/ioctl.h>

#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#ifdef __x86_64__
extern "C" int kvm_set_sregs(int vcpu, const kvm_sregs *regs)
{
    return ioctl(vcpu, KVM_SET_SREGS, regs);
}

extern "C" int kvm_translate(int vcpu, kvm_translation *arg) {
    return ioctl(vcpu, KVM_TRANSLATE, arg);
}
#endif
