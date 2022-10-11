#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "semaphore.hpp"

using namespace uplift;
using namespace uplift::objects;
using namespace uplift::syscall_errors;

Semaphore::Semaphore(Runtime* runtime)
  : Object(runtime, ObjectType)
{
}

Semaphore::~Semaphore()
{
}

SCERR Semaphore::Initialize(uint32_t flags, uint32_t arg3, uint32_t arg4)
{
  flags_ = flags;
  arg3_ = arg3;
  arg4_ = arg4;
  return SUCCESS;
}

SCERR Semaphore::Close()
{
  return SUCCESS;
}
