#include "stdafx.h"

#include <xenia/base/memory.h>
#include <xenia/base/string.h>

#include "runtime.hpp"
#include "syscalls.hpp"
#include "syscall_errors.hpp"
#include "helpers.hpp"

#include <windows.h>

using namespace uplift;
using namespace uplift::syscall_errors;

SCERR clock_gettime_win(uint32_t clock_id, void* tp)
{
  struct timespec
  {
    int64_t tv_sec;
    int32_t tv_nsec;
  };

  auto tsp = static_cast<timespec*>(tp);

  FILETIME filetime;
  GetSystemTimePreciseAsFileTime(&filetime);
  uint64_t value = ((uint64_t)filetime.dwHighDateTime << 32) | (uint64_t)filetime.dwLowDateTime;
  value -= 11644473600000000ull;
  tsp->tv_sec = value / 10000000;
  tsp->tv_nsec = value % 10000000;
  return SUCCESS;
}
