#include <linux/kvm.h>

#include <sys/ioctl.h>

#include <errno.h>

extern "C" int kvm_check_version(int kvm, bool *compat)
{
    auto v = ioctl(kvm, KVM_GET_API_VERSION);

    if (v < 0) {
        return errno;
    }

    *compat = (v == KVM_API_VERSION);
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
