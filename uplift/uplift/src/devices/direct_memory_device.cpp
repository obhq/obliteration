#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "direct_memory_device.hpp"

using namespace uplift;
using namespace uplift::devices;
using namespace uplift::syscall_errors;

DirectMemoryDevice::DirectMemoryDevice(Runtime* runtime)
  : Device(runtime)
  , path_()
  , flags_(0)
  , mode_(0)
  , is_initialized_(false)
{
}

DirectMemoryDevice::~DirectMemoryDevice()
{
}

SCERR DirectMemoryDevice::Initialize(std::string path, uint32_t flags, uint32_t mode)
{
  path_ = path;
  flags_ = flags;
  mode_ = mode;
  is_initialized_ = true;
  return SUCCESS;
}

SCERR DirectMemoryDevice::Close()
{
  return SUCCESS;
}

SCERR DirectMemoryDevice::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR DirectMemoryDevice::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR DirectMemoryDevice::IOControl(uint32_t request, void* argp)
{
  switch (request)
  {
    case 0x4008800Au: // get size
    {
      *static_cast<size_t*>(argp) = 0xBADC0FFEE0DDF00Dull;
      return SUCCESS;
    }

    case 0xC0288001u: // allocate
    {
      struct request_args
      {
        void* allocation;
        void* unknown_08;
        size_t size;
        size_t alignment;
        uint32_t unknown_20;
      };
      auto args = static_cast<request_args*>(argp);
      printf("ALLOCATE DIRECT MEMORY: %p %p %I64x %I64x %u\n", args->allocation, args->unknown_08, args->size, args->alignment, args->unknown_20);
      assert_true(args->unknown_08 == reinterpret_cast<void*>(0xBADC0FFEE0DDF00Dull));
      args->allocation = reinterpret_cast<void*>(0xBADC0FFEE0DDF00Dull);
      return SUCCESS;
    }
  }

  assert_always();
  return SCERR::eNOSYS;
}

SCERR DirectMemoryDevice::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SCERR::eNOSYS;
}
