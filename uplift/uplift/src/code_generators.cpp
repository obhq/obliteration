#include "stdafx.h"

#include <xenia/base/assert.h>

#include "code_generators.hpp"
#include "rip_pointers.hpp"

using namespace uplift::code_generators;

Xbyak::Operand::Code capstone_to_xbyak(x86_reg reg)
{
#define CASE_R(x) \
  case X86_REG_E ## x: \
  case X86_REG_R ## x: \
  { \
    return Xbyak::Operand::R ## x; \
  }
#define CASE_N(x) \
  case X86_REG_R ## x ## D: \
  case X86_REG_R ## x: \
  { \
    return Xbyak::Operand::R ## x; \
  }
  switch (reg)
  {
    CASE_R(AX)
    CASE_R(CX)
    CASE_R(DX)
    CASE_R(BX)
    CASE_R(SP)
    CASE_R(BP)
    CASE_R(SI)
    CASE_R(DI)
    CASE_N(8)
    CASE_N(9)
    CASE_N(10)
    CASE_N(11)
    CASE_N(12)
    CASE_N(13)
    CASE_N(14)
    CASE_N(15)
  }
  assert_always();
  return Xbyak::Operand::Code::RAX;
#undef CASE_N
#undef CASE_R
}

SyscallTrampolineGenerator::SyscallTrampolineGenerator()
{
  Xbyak::Label rip_pointers_pointer;
  Xbyak::Label target_pointer;

  // runtime -> RCX
  // syscall id -> RDX
  // RDI, RSI, RDX, R10(RCX), R8, R9 -> R8, R9, stack(0), stack(1), stack(3), stack(4)

  // nonvolatile: RBP, RSP, RBX, R12, R13, R14, R15
  // volatile: RCX, R11

  push(rbp);
  mov(rbp, rsp);

  // fix stack alignment, no guarantee it is 16-byte aligned when jumping from random code
  and_(rsp, ~15u);
  push(rbp);

  push(r12); push(r13); push(r14); push(r15);
  push(rbx);

  sub(rsp, 8); // result storage
  mov(qword[rsp], 0);
  push(rsp); // push address of result

  Xbyak::Label label1, label2;

  cmp(rax, 0);
  jz(label1);

  push(r9);
  push(r8);
  push(rcx);
  push(rdx);
  mov(r9, rsi);
  mov(r8, rdi);
  mov(rdx, rax);
  jmp(label2);

  L(label1);
  push(0);
  push(r9);
  push(r8);
  push(rcx);
  mov(r9, rdx);
  mov(r8, rsi);
  mov(rdx, rdi);

  L(label2);

  mov(rcx, ptr[rip + rip_pointers_pointer]);
  mov(rcx, ptr[rcx + offsetof(RIPPointers, runtime)]);

  push(r9); // SHADOW SPACE
  push(r8); // SHADOW SPACE
  push(rdx); // SHADOW SPACE
  push(rcx); // SHADOW SPACE

  mov(rax, ptr[rip + rip_pointers_pointer]);
  call(ptr[rax + offsetof(RIPPointers, syscall_handler)]);

  add(rsp, (4 + 2 + 4) * sizeof(void*));

  sub(al, 1); // set CF on error
  mov(rax, ptr[rsp - 8]);

  pop(rbx);
  pop(r15); pop(r14); pop(r13); pop(r12);
  pop(rsp);
  pop(rbp);

  jmp(ptr[rip + target_pointer]);

  assert_true(sizeof(Tail) == 16);
  L(target_pointer);
  dq(0ULL);
  L(rip_pointers_pointer);
  dq(0ULL);
}

NakedSyscallTrampolineGenerator::NakedSyscallTrampolineGenerator(uint64_t syscall_id)
{
  Xbyak::Label target_pointer;
  Xbyak::Label rip_pointers_pointer;

  // runtime -> RCX
  // syscall id -> RDX
  // RDI, RSI, RDX, R10, R8, R9 -> R8, R9, stack(0), stack(1), stack(3), stack(4)

  // nonvolatile: RBP, RSP, RBX, R12, R13, R14, R15
  // volatile: RCX, R11

  push(rbp);
  mov(rbp, rsp);

  // fix stack alignment, no guarantee it is 16-byte aligned when jumping from random code
  and_(rsp, ~15u);
  push(rbp);

  push(r12); push(r13); push(r14); push(r15);
  push(rbx);

  sub(rsp, 8); // result storage
  mov(qword[rsp], 0);
  push(rsp); // push address of result

  if (syscall_id != 0)
  {
    push(r9);
    push(r8);
    push(r10);
    push(rdx);
    mov(r9, rsi);
    mov(r8, rdi);
    mov(rdx, syscall_id);
  }
  else
  {
    push(0);
    push(r9);
    push(r8);
    push(r10);
    mov(r9, rdx);
    mov(r8, rsi);
    mov(rdx, rdi); // syscall id comes in as argument
  }

  mov(rcx, ptr[rip + rip_pointers_pointer]);
  mov(rcx, ptr[rcx + offsetof(RIPPointers, runtime)]);

  push(r9); // SHADOW SPACE
  push(r8); // SHADOW SPACE
  push(rdx); // SHADOW SPACE
  push(rcx); // SHADOW SPACE

  mov(rax, ptr[rip + rip_pointers_pointer]);
  call(ptr[rax + offsetof(RIPPointers, syscall_handler)]);

  add(rsp, (4 + 2 + 4) * sizeof(void*));

  sub(al, 1); // set CF on error
  mov(rax, ptr[rsp - 8]);

  pop(rbx);
  pop(r15); pop(r14); pop(r13); pop(r12);
  pop(rsp);
  pop(rbp);

  jmp(ptr[rip + target_pointer]);

  assert_true(sizeof(Tail) == 16);
  L(target_pointer);
  dq(0ULL);
  L(rip_pointers_pointer);
  dq(0ULL);
}

FSBaseMovGenerator::FSBaseMovGenerator(x86_reg reg, uint8_t reg_size, int64_t disp)
{
  Xbyak::Label target_pointer;
  Xbyak::Label rip_pointers_pointer;

  assert_true(reg_size == 8 || reg_size == 4);
  auto xbyak_reg = Xbyak::Reg64(capstone_to_xbyak(reg));

  mov(xbyak_reg, ptr[rip + rip_pointers_pointer]);
  mov(xbyak_reg, ptr[xbyak_reg + offsetof(RIPPointers, fsbase)]);
  if (disp != 0)
  {
    assert_true(disp >= INT32_MIN && disp <= INT32_MAX);
    add(xbyak_reg, static_cast<Xbyak::uint32>(disp));
  }

  if (reg_size == 4)
  {
    mov(xbyak_reg.cvt32(), ptr[xbyak_reg]);
  }
  else
  {
    mov(xbyak_reg, ptr[xbyak_reg]);
  }

  jmp(ptr[rip + target_pointer]);
  assert_true(sizeof(Tail) == 16);
  L(target_pointer);
  dq(0ULL);
  L(rip_pointers_pointer);
  dq(0ULL);
}
