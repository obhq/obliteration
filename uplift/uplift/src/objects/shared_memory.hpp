#pragma once

#include "object.hpp"

namespace uplift::objects
{
  class SharedMemory : public Object
  {
  public:
    static const Object::Type ObjectType = Type::SharedMemory;

  public:
    SharedMemory(Runtime* runtime);
    virtual ~SharedMemory();

    SyscallError Initialize(const std::string& path, uint32_t flags, uint16_t mode);

    SyscallError Close();
    SyscallError Read(void* data_buffer, size_t data_size, size_t* read_size);
    SyscallError Write(const void* data_buffer, size_t data_size, size_t* written_size);
    SyscallError Truncate(int64_t length);
    SyscallError IOControl(uint32_t request, void* argp);
    SyscallError MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation);

  private:
    void* native_handle_;
    int64_t length_;
    std::string path_;
    uint32_t flags_;
    uint16_t mode_;
  };
}
