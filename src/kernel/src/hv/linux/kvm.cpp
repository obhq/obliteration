// The reason we need C++ here is because the KVM ioctls is not available in the libc binding.
#include <linux/kvm.h>

#include <sys/ioctl.h>

#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

extern "C" int kvm_check_version(int kvm, bool *compat)
{
    auto v = ioctl(kvm, KVM_GET_API_VERSION);

    if (v < 0) {
        return errno;
    }

    *compat = (v == KVM_API_VERSION);
    return 0;
}

extern "C" int kvm_max_vcpus(int kvm, size_t *max)
{
    auto num = ioctl(kvm, KVM_CHECK_EXTENSION, KVM_CAP_MAX_VCPUS);

    if (num < 0) {
        return errno;
    }

    *max = static_cast<size_t>(num);
    return 0;
}

extern "C" int kvm_create_vm(int kvm, int *fd)
{
    auto vm = ioctl(kvm, KVM_CREATE_VM, 0);

    if (vm < 0) {
        return errno;
    }

    *fd = vm;
    return 0;
}

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

extern "C" int kvm_get_vcpu_mmap_size(int kvm)
{
    return ioctl(kvm, KVM_GET_VCPU_MMAP_SIZE);
}

extern "C" int kvm_create_vcpu(int vm, int id, int *fd)
{
    auto vcpu = ioctl(vm, KVM_CREATE_VCPU, id);

    if (vcpu < 0) {
        return errno;
    }

    *fd = vcpu;

    return 0;
}

extern "C" int kvm_run(int vcpu)
{
    return ioctl(vcpu, KVM_RUN);
}

extern "C" int kvm_get_regs(int vcpu, kvm_regs *regs)
{
    return ioctl(vcpu, KVM_GET_REGS, regs);
}

extern "C" int kvm_set_regs(int vcpu, kvm_regs *regs)
{
    return ioctl(vcpu, KVM_SET_REGS, regs);
}
