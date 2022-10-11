#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "notification_device.hpp"

using namespace uplift;
using namespace uplift::devices;
using namespace uplift::syscall_errors;

NotificationDevice::NotificationDevice(Runtime* runtime)
  : Device(runtime)
{
}

NotificationDevice::~NotificationDevice()
{
}

SCERR NotificationDevice::Initialize(std::string path, uint32_t flags, uint32_t mode)
{
  return SUCCESS;
}

SCERR NotificationDevice::Close()
{
  return SUCCESS;
}

SCERR NotificationDevice::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR NotificationDevice::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  if (data_size > 0x28)
  {
    printf("NOTIFICATION: %s\n", &static_cast<const char*>(data_buffer)[0x28]);
  }

  if (written_size)
  {
    *written_size = data_size;
  }

  return SUCCESS;
}

SCERR NotificationDevice::IOControl(uint32_t request, void* argp)
{
  assert_always();
  return SCERR::eNOSYS;
}

SCERR NotificationDevice::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SCERR::eNOSYS;
}
