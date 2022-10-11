#pragma once

#include <cstdint>

#include <capstone/capstone.h>
#include <capstone/x86.h>

#include <xbyak/xbyak.h>

#include "rip_pointers.hpp"

namespace uplift::code_generators
{
  class SyscallTrampolineGenerator : public Xbyak::CodeGenerator
  {
  public:
    SyscallTrampolineGenerator();

    struct Tail
    {
      void* target;
      RIPPointers* rip_pointers;
    };
  };

  class NakedSyscallTrampolineGenerator : public Xbyak::CodeGenerator
  {
  public:
    NakedSyscallTrampolineGenerator(uint64_t syscall_id);

    struct Tail
    {
      void* target;
      RIPPointers* rip_pointers;
    };
  };

  class FSBaseMovGenerator : public Xbyak::CodeGenerator
  {
  public:
    FSBaseMovGenerator(x86_reg reg, uint8_t reg_size, int64_t disp);

    struct Tail
    {
      void* target;
      RIPPointers* rip_pointers;
    };
  };
}
