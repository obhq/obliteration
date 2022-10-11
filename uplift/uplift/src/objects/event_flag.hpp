#pragma once

#include "object.hpp"

namespace uplift::objects
{
  class EventFlag : public Object
  {
  public:
    static const Object::Type ObjectType = Type::EventFlag;

  public:
    EventFlag(Runtime* runtime);
    virtual ~EventFlag();

    SyscallError Initialize(uint32_t flags, uint64_t arg3);

    SyscallError Close();
    SyscallError Read(void* data_buffer, size_t data_size, size_t* read_size);
    SyscallError Write(const void* data_buffer, size_t data_size, size_t* written_size);
    SyscallError IOControl(uint32_t request, void* argp);
    SyscallError MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation);

  private:
    uint32_t flags_;
    uint64_t arg3_;
  };
}
