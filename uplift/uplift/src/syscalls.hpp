#pragma once

namespace uplift
{
  class Runtime;

  enum class SyscallError : uint64_t;

  union SyscallReturnValue
  {
    void* ptr;
    uint64_t val;
    SyscallError err;

    /* To ensure the entire union is written, SyscallError
     * is forced to 64-bit width.
     */
  };

  typedef bool(*SYSCALL_HANDLER)(Runtime* runtime, SyscallReturnValue& retval, ...);

  class SYSCALLS
  {
  private:
    SYSCALLS() {}
  public:
#define SYSCALL(x, y, ...) static bool y(Runtime* runtime, SyscallReturnValue&, __VA_ARGS__)
#include "syscall_table.inl"
#undef SYSCALL
  };

  class Runtime;

  struct SyscallEntry
  {
    void* handler;
    const char* name;
  };

  const size_t SyscallTableSize = 1024;
  void get_syscall_table(SyscallEntry table[SyscallTableSize]);
}
