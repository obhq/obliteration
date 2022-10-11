#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "console_device.hpp"

using namespace uplift;
using namespace uplift::devices;
using namespace uplift::syscall_errors;

ConsoleDevice::ConsoleDevice(Runtime* runtime)
  : Device(runtime)
{
}

ConsoleDevice::~ConsoleDevice()
{
}

SCERR ConsoleDevice::Initialize(std::string path, uint32_t flags, uint32_t mode)
{
  return SUCCESS;
}

SCERR ConsoleDevice::Close()
{
  return SUCCESS;
}

SCERR ConsoleDevice::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR ConsoleDevice::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR ConsoleDevice::IOControl(uint32_t request, void* argp)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR ConsoleDevice::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SCERR::eNOSYS;
}
