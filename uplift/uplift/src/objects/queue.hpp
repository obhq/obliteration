#pragma once

#include "object.hpp"

namespace uplift::objects
{
  class Queue : public Object
  {
  public:
    static const Object::Type ObjectType = Type::Queue;

  public:
    Queue(Runtime* runtime);
    virtual ~Queue();

    SyscallError Close();
  };
}
