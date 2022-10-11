#pragma once

#include "object.hpp"

namespace uplift::objects
{
  class Eport : public Object
  {
  public:
    static const Object::Type ObjectType = Type::Eport;

  public:
    Eport(Runtime* runtime);
    virtual ~Eport();

    SyscallError Close();
    SyscallError Read(void* data_buffer, size_t data_size, size_t* read_size);
    SyscallError Write(const void* data_buffer, size_t data_size, size_t* written_size);
    SyscallError IOControl(uint32_t request, void* argp);
    SyscallError MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation);
  };
}
