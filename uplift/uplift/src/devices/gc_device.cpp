#include "stdafx.h"

#include <xenia/base/memory.h>

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "gc_device.hpp"

using namespace uplift;
using namespace uplift::devices;
using namespace uplift::syscall_errors;

GCDevice::GCDevice(Runtime* runtime)
  : Device(runtime)
{
}

GCDevice::~GCDevice()
{
}

SCERR GCDevice::Initialize(std::string path, uint32_t flags, uint32_t mode)
{
  return SUCCESS;
}

SCERR GCDevice::Close()
{
  return SUCCESS;
}

SCERR GCDevice::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR GCDevice::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR GCDevice::IOControl(uint32_t request, void* argp)
{
  switch (request)
  {
    case 0xC008811Bu:
    {
      struct request_args
      {
        uint64_t unknown;
      };
      auto args = static_cast<request_args*>(argp);
      printf("gc ioctl(%x): %I64x\n", request, args->unknown);
      args->unknown = 0x1234FFFF00000000ull;
      return SUCCESS;
    }

    case 0xC00C8110u: // "set gs ring sizes"
    {
      struct request_args
      {
        uint32_t unknown_0;
        uint32_t unknown_4;
        uint32_t unknown_8;
      };
      auto args = static_cast<request_args*>(argp);
      printf("gc ioctl(%x): %x, %x, %x\n", request, args->unknown_0, args->unknown_4, args->unknown_8);
      return SUCCESS;
    }

    case 0xC0848119u:
    {
      struct request_args
      {
        uint32_t unknown_00;
        uint32_t unknown_04;
        uint32_t unknown_08;
        uint32_t unknown_0C;
        uint8_t unknown_10[112];
        uint32_t unknown_80;
      };
      auto args = static_cast<request_args*>(argp);
      printf("gc ioctl(%x): %x, %x, %x, %x, %x\n", request, args->unknown_00, args->unknown_04, args->unknown_08, args->unknown_0C, args->unknown_80);
      return SUCCESS;
    }
  }

  assert_always();
  return SCERR::eNOSYS;
}

SCERR GCDevice::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_true(!(flags & ~(0x1 | 0x2 | 0x10 | 0x1000 | 0x2000)));

  auto access = xe::memory::PageAccess::kReadWrite;
  auto allocation_type = xe::memory::AllocationType::kReserveCommit;

  // fake it, for now
  allocation = xe::memory::AllocFixed(addr, len, allocation_type, access);
  if (!allocation && !(flags & 0x10))
  {
    // not fixed, try allocating again
    allocation = xe::memory::AllocFixed(nullptr, len, allocation_type, access);
  }

  if (allocation)
  {
    return SUCCESS;
  }

  return SCERR::eNOMEM;
}
