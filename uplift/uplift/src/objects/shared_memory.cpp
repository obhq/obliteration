#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "shared_memory.hpp"

#include <windows.h>

using namespace uplift;
using namespace uplift::objects;
using namespace uplift::syscall_errors;

SharedMemory::SharedMemory(Runtime* runtime)
  : Object(runtime, ObjectType)
  , native_handle_(INVALID_HANDLE_VALUE)
  , length_(0)
  , path_()
  , flags_(0)
  , mode_(0)
{
}

SharedMemory::~SharedMemory()
{
  Close();
}

SCERR SharedMemory::Initialize(const std::string& path, uint32_t flags, uint16_t mode)
{
  path_ = path;
  flags_ = flags;
  mode_ = mode;
  return SUCCESS;
}

SCERR SharedMemory::Close()
{
  if (native_handle_ != INVALID_HANDLE_VALUE)
  {
    CloseHandle(native_handle_);
    native_handle_ = INVALID_HANDLE_VALUE;
  }

  return SUCCESS;
}

SCERR SharedMemory::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR SharedMemory::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR SharedMemory::Truncate(int64_t length)
{
  DWORD size_lo = length & UINT32_MAX;
  DWORD size_hi = (length >> 32) & UINT32_MAX;

  auto new_native_handle = CreateFileMapping(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE | SEC_COMMIT, size_hi, size_lo, nullptr);

  if (native_handle_ != INVALID_HANDLE_VALUE)
  {
    if (length_ > 0 && length_ > 0)
    {
      auto src = MapViewOfFile(native_handle_, PAGE_READONLY, 0, 0, length_);
      assert_not_null(src);
      auto dst = MapViewOfFile(new_native_handle, PAGE_READWRITE, 0, 0, length);
      assert_not_null(dst);
      memcpy(dst, src, std::min(length, length_));
      UnmapViewOfFile(dst);
      UnmapViewOfFile(src);
    }
    CloseHandle(native_handle_);
  }

  native_handle_ = new_native_handle;
  length_ = length;
  return SUCCESS;
}

SCERR SharedMemory::IOControl(uint32_t request, void* argp)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR SharedMemory::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  DWORD access;

  switch (prot & (1 | 2 | 4))
  {
    case 1:
    {
      access = FILE_MAP_READ;
      break;
    }

    case 2:
    {
      access = FILE_MAP_WRITE;
      break;
    }

    case 3:
    {
      access = FILE_MAP_READ | FILE_MAP_WRITE;
      break;
    }

    default:
    {
      assert_always();
      access = FILE_MAP_READ | FILE_MAP_WRITE;
      break;
    }
  }

  DWORD offset_hi = static_cast<DWORD>((offset >> 32) & UINT32_MAX);
  DWORD offset_lo = static_cast<DWORD>((offset >> 0) & UINT32_MAX);

  allocation = MapViewOfFileEx(native_handle_, access, offset_hi, offset_lo, len, addr);
  return allocation != nullptr ? SUCCESS : SCERR::eNOMEM;
}
