#include "stdafx.h"

#include "../runtime.hpp"
#include "file.hpp"

using namespace uplift;
using namespace uplift::objects;

File::File(Runtime* runtime)
  : Object(runtime, ObjectType)
{
}

File::~File()
{
}
