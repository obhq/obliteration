#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "event_flag.hpp"

using namespace uplift;
using namespace uplift::objects;
using namespace uplift::syscall_errors;

EventFlag::EventFlag(Runtime* runtime)
  : Object(runtime, ObjectType)
{
}

EventFlag::~EventFlag()
{
}

SCERR EventFlag::Initialize(uint32_t flags, uint64_t arg3)
{
  flags_ = flags;
  arg3_ = arg3;
  return SUCCESS;
}

SCERR EventFlag::Close()
{
  return SUCCESS;
}

SCERR EventFlag::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR EventFlag::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR EventFlag::IOControl(uint32_t request, void* argp)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR EventFlag::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SCERR::eNODEV;
}
