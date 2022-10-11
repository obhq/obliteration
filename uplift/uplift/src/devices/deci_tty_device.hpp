#pragma once

#include "device.hpp"

namespace uplift::devices
{
  class DeciTTYDevice : public Device
  {
  public:
    DeciTTYDevice(Runtime* runtime);
    virtual ~DeciTTYDevice();

    SyscallError Initialize(std::string path, uint32_t flags, uint32_t mode);

    SyscallError Close();
    SyscallError Read(void* data_buffer, size_t data_size, size_t* read_size);
    SyscallError Write(const void* data_buffer, size_t data_size, size_t* written_size);
    SyscallError IOControl(uint32_t request, void* argp);
    SyscallError MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation);
  };
}
