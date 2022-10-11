#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "dipsw_device.hpp"

using namespace uplift;
using namespace uplift::devices;
using namespace uplift::syscall_errors;

DipswDevice::DipswDevice(Runtime* runtime)
  : Device(runtime)
{
}

DipswDevice::~DipswDevice()
{
}

SCERR DipswDevice::Initialize(std::string path, uint32_t flags, uint32_t mode)
{
  return SUCCESS;
}

SCERR DipswDevice::Close()
{
  return SUCCESS;
}

SCERR DipswDevice::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR DipswDevice::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR DipswDevice::IOControl(uint32_t request, void* argp)
{
  switch (request)
  {
    case 0x40048806u:
    {
      *static_cast<uint32_t*>(argp) = 1;
      return SUCCESS;
    }
    case 0x40048807u:
    {
      *static_cast<uint32_t*>(argp) = 0;
      return SUCCESS;
    }
    case 0x40088808u:
    {
      *static_cast<uint64_t*>(argp) = 0;
      return SUCCESS;
    }

    case 0x40088809u:
    {
      *static_cast<uint64_t*>(argp) = 0;
      return SUCCESS;
    }
  }
  assert_always();
  return SCERR::eINVAL;
}

SCERR DipswDevice::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SCERR::eNOSYS;
}
