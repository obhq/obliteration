#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "eport.hpp"

using namespace uplift;
using namespace uplift::objects;
using namespace uplift::syscall_errors;

Eport::Eport(Runtime* runtime)
  : Object(runtime, ObjectType)
{
}

Eport::~Eport()
{
}

SCERR Eport::Close()
{
  return SUCCESS;
}

SCERR Eport::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR Eport::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR Eport::IOControl(uint32_t request, void* argp)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR Eport::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SCERR::eNODEV;
}
