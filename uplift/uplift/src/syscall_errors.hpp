#pragma once

#include "syscall_error_table.hpp"

namespace uplift
{
  namespace syscall_errors
  {
    using SCERR = uplift::SyscallError;
    const SCERR SUCCESS = SCERR::SUCCESS;
    bool inline IS_ERROR(SCERR err) { return err != SUCCESS; }
    bool inline IS_SUCCESS(SCERR err) { return err == SCERR::SUCCESS; }
  }
}
