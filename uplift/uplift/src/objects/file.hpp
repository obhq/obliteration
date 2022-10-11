#pragma once

#include "object.hpp"

namespace uplift::objects
{
  class File : public Object
  {
  public:
    static const Object::Type ObjectType = Type::File;

  protected:
    File(Runtime* runtime);

  public:
    virtual ~File();
  };
}
