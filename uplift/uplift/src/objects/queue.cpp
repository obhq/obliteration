#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "queue.hpp"

using namespace uplift;
using namespace uplift::objects;
using namespace uplift::syscall_errors;

Queue::Queue(Runtime* runtime)
  : Object(runtime, ObjectType)
{
}

Queue::~Queue()
{
}

SCERR Queue::Close()
{
  return SUCCESS;
}
