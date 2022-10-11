#include "stdafx.h"

#include "../runtime.hpp"
#include "device.hpp"

using namespace uplift;
using namespace uplift::devices;

Device::Device(Runtime* runtime)
  : Object(runtime, ObjectType)
{
}

Device::~Device()
{
}
