#pragma once

#include "object.hpp"

namespace uplift::objects
{
  class Semaphore : public Object
  {
  public:
    static const Object::Type ObjectType = Type::Semaphore;

  public:
    Semaphore(Runtime* runtime);
    virtual ~Semaphore();

    SyscallError Initialize(uint32_t flags, uint32_t arg3, uint32_t arg4);

    SyscallError Close();

  private:
    uint32_t flags_;
    uint32_t arg3_;
    uint32_t arg4_;
  };
}
