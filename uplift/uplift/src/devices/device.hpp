#pragma once

#include "../objects/object.hpp"

namespace uplift::devices
{
  class Device : public objects::Object
  {
  public:
    static const Object::Type ObjectType = Type::Device;

  protected:
    Device(Runtime* runtime);

  public:
    virtual ~Device();

    virtual SyscallError Initialize(std::string path, uint32_t flags, uint32_t mode) = 0;
  };
}
