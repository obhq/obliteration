#include "stdafx.h"

#include <xenia/base/assert.h>

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "object.hpp"

using namespace uplift;
using namespace uplift::objects;
using namespace uplift::syscall_errors;

Object::Object(Runtime* runtime, Type type)
  : runtime_(runtime)
  , handles_()
  , pointer_ref_count_(1)
  , type_(type)
{
  handles_.reserve(10);
  runtime->object_table()->AddHandle(this, nullptr);
}

Object::~Object()
{
  assert_zero(pointer_ref_count_);
}

void Object::RetainHandle() 
{
  runtime_->object_table()->RetainHandle(handles_[0]);
}

bool Object::ReleaseHandle()
{
  return runtime_->object_table()->ReleaseHandle(handles_[0]);
}

void Object::Retain() { ++pointer_ref_count_; }

void Object::Release()
{
  if (--pointer_ref_count_ == 0)
  {
    delete this;
  }
}

uint32_t Object::Delete()
{
  if (!name_.empty())
  {
    runtime_->object_table()->RemoveNameMapping(name_);
  }
  return runtime_->object_table()->RemoveHandle(handles_[0]);
}

SCERR Object::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR Object::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR Object::Truncate(int64_t length)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR Object::IOControl(uint32_t request, void* argp)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR Object::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SCERR::eNODEV;
}
