#include "stdafx.h"

#include "bmi1.hpp"

// https://www.youtube.com/watch?v=pc0mxOXbWIU

uint64_t read_register(x86_op_type reg, xe::X64Context* thread_context)
{
#define CASE_R(x,y) \
  case X86_REG_R ## x: return thread_context->r ## y; \
  case X86_REG_E ## x: return (uint32_t)thread_context->r ## y;
#define CASE_N(x) \
  case X86_REG_R ## x: return thread_context->r ## x; \
  case X86_REG_R ## x ## D: return (uint32_t)thread_context->r ## x;
  switch (reg)
  {
    CASE_R(AX, ax);
    CASE_R(CX, cx);
    CASE_R(DX, dx);
    CASE_R(BX, bx);
    CASE_R(SP, sp);
    CASE_R(BP, bp);
    CASE_R(SI, si);
    CASE_R(DI, di);
    CASE_N(8);
    CASE_N(9);
    CASE_N(10);
    CASE_N(11);
    CASE_N(12);
    CASE_N(13);
    CASE_N(14);
    CASE_N(15);
  }
#undef CASE_N
#undef CASE_R
  assert_always();
  return 0;
}

uint64_t read_operand(cs_x86_op op, xe::X64Context* thread_context)
{
  if (op.type == X86_OP_REG)
  {
#define CASE_R(x,y) \
  case X86_REG_R ## x: return thread_context->r ## y; \
  case X86_REG_E ## x: return (uint32_t)thread_context->r ## y;
#define CASE_N(x) \
  case X86_REG_R ## x: return thread_context->r ## x; \
  case X86_REG_R ## x ## D: return (uint32_t)thread_context->r ## x;
    switch (op.reg)
    {
      CASE_R(AX, ax);
      CASE_R(CX, cx);
      CASE_R(DX, dx);
      CASE_R(BX, bx);
      CASE_R(SP, sp);
      CASE_R(BP, bp);
      CASE_R(SI, si);
      CASE_R(DI, di);
      CASE_N(8);
      CASE_N(9);
      CASE_N(10);
      CASE_N(11);
      CASE_N(12);
      CASE_N(13);
      CASE_N(14);
      CASE_N(15);
    }
#undef CASE_N
#undef CASE_R
  }
  else if (op.type == X86_OP_MEM)
  {
    if (op.mem.segment == X86_REG_INVALID)
    {
      auto address = read_register((x86_op_type)op.mem.base, thread_context);
      address += static_cast<uint64_t>(op.mem.disp);
      if (op.mem.index != X86_REG_INVALID)
      {
        auto index_value = read_register((x86_op_type)op.mem.index, thread_context);
        index_value *= op.mem.scale;
        address += static_cast<uint64_t>(index_value);
      }
      if (op.size == 8)
      {
        return *reinterpret_cast<uint64_t*>(address);
      }
      else if (op.size == 4)
      {
        return *reinterpret_cast<uint32_t*>(address);
      }
    }
    else
    {
      assert_always();
    }
  }
  assert_always();
  return 0;
}

void write_operand(cs_x86_op op, xe::X64Context* thread_context, uint64_t value)
{
#define CASE_R(x,y) \
  case X86_REG_R ## x: \
  { \
    thread_context->r##y = value; \
    return; \
  } \
  case X86_REG_E ## x: \
  { \
    thread_context->r##y &= ~UINT32_MAX; \
    thread_context->r##y |= (uint32_t)value; \
    return; \
  }
#define CASE_N(x) \
  case X86_REG_R ## x: \
  { \
    thread_context->r##x = value; \
    return; \
  } \
  case X86_REG_R ## x ## D: \
  { \
    thread_context->r##x &= ~UINT32_MAX; \
    thread_context->r##x |= (uint32_t)value; \
    return; \
  }
  if (op.type == X86_OP_REG)
  {
    switch (op.reg)
    {
      CASE_R(AX, ax);
      CASE_R(CX, cx);
      CASE_R(DX, dx);
      CASE_R(BX, bx);
      CASE_R(SP, sp);
      CASE_R(BP, bp);
      CASE_R(SI, si);
      CASE_R(DI, di);
      CASE_N(8);
      CASE_N(9);
      CASE_N(10);
      CASE_N(11);
      CASE_N(12);
      CASE_N(13);
      CASE_N(14);
      CASE_N(15);
    }
  }
  assert_always();
#undef CASE_N
#undef CASE_R
}

#define UPDATE_EFLAGS(x,y) \
  if (x) thread_context->eflags |= 1u << y; \
  else thread_context->eflags &= ~(1u << y);

#define UPDATE_CF(x) UPDATE_EFLAGS(x, 0)
#define UPDATE_ZF(x) UPDATE_EFLAGS(x, 6)
#define UPDATE_SF(x) UPDATE_EFLAGS(x, 7)
#define UPDATE_OF(x) UPDATE_EFLAGS(x, 11)

void simulate_andn(cs_insn* insn, xe::X64Context* thread_context)
{
  assert_true(insn->detail->x86.op_count == 3);

  auto src1 = read_operand(insn->detail->x86.operands[1], thread_context);
  auto src2 = read_operand(insn->detail->x86.operands[2], thread_context);
  auto result = (~src1) & src2;
  write_operand(insn->detail->x86.operands[0], thread_context, result);

  bool is_zero, is_sign;
  if (insn->detail->x86.operands[0].size == 4)
  {
    is_sign = (result & 0x80000000ull) != 0;
    is_zero = (result & UINT32_MAX) == 0;
  }
  else if (insn->detail->x86.operands[0].size == 8)
  {
    is_sign = (result & 0x8000000000000000ull) != 0;
    is_zero = result == 0;
  }
  else
  {
    assert_always();
  }

  UPDATE_CF(false);
  UPDATE_ZF(is_zero);
  UPDATE_SF(is_sign);
  UPDATE_OF(false);

  thread_context->rip += insn->size;
}

void simulate_bextr(cs_insn* insn, xe::X64Context* thread_context)
{
  assert_true(insn->detail->x86.op_count == 3);

  auto src1 = read_operand(insn->detail->x86.operands[1], thread_context);
  auto src2 = read_operand(insn->detail->x86.operands[2], thread_context);

  auto start = src2 & 0xFF;
  auto len = (src2 >> 8) & 0xFF;

  auto result = (src1 >> start) & ((1 << len) - 1);
  write_operand(insn->detail->x86.operands[0], thread_context, result);

  bool is_zero;
  if (insn->detail->x86.operands[0].size == 4)
  {
    is_zero = (result & UINT32_MAX) == 0;
  }
  else if (insn->detail->x86.operands[0].size == 8)
  {
    is_zero = result == 0;
  }
  else
  {
    assert_always();
  }

  UPDATE_CF(false);
  UPDATE_ZF(is_zero);
  UPDATE_SF(false);
  UPDATE_OF(false);

  thread_context->rip += insn->size;
}

void simulate_blsi(cs_insn* insn, xe::X64Context* thread_context)
{
  assert_true(insn->detail->x86.op_count == 2);

  auto src1 = read_operand(insn->detail->x86.operands[1], thread_context);
#pragma warning(suppress: 4146)
  auto result = (-src1) & src1;
  write_operand(insn->detail->x86.operands[0], thread_context, result);

  bool is_carry, is_zero, is_sign;
  if (insn->detail->x86.operands[1].size == 4)
  {
    is_carry = (src1 & UINT32_MAX) == 0;
  }
  else if (insn->detail->x86.operands[1].size == 8)
  {
    is_carry = src1 == 0;
  }
  else
  {
    assert_always();
  }
  if (insn->detail->x86.operands[0].size == 4)
  {
    is_zero = (result & UINT32_MAX) == 0;
    is_sign = (result & 0x80000000ull) != 0;
  }
  else if (insn->detail->x86.operands[0].size == 8)
  {
    is_zero = result == 0;
    is_sign = (result & 0x8000000000000000ull) != 0;
  }
  else
  {
    assert_always();
  }

  UPDATE_CF(is_carry);
  UPDATE_ZF(is_zero);
  UPDATE_SF(is_sign);
  UPDATE_OF(false);

  thread_context->rip += insn->size;
}

void simulate_blsr(cs_insn* insn, xe::X64Context* thread_context)
{
  assert_true(insn->detail->x86.op_count == 2);

  auto src1 = read_operand(insn->detail->x86.operands[1], thread_context);
#pragma warning(suppress: 4146)
  auto result = (src1 - 1) & src1;
  write_operand(insn->detail->x86.operands[0], thread_context, result);

  bool is_carry, is_zero, is_sign;
  if (insn->detail->x86.operands[1].size == 4)
  {
    is_carry = (src1 & UINT32_MAX) == 0;
  }
  else if (insn->detail->x86.operands[1].size == 8)
  {
    is_carry = src1 == 0;
  }
  else
  {
    assert_always();
  }
  if (insn->detail->x86.operands[0].size == 4)
  {
    is_zero = (result & UINT32_MAX) == 0;
    is_sign = (result & 0x80000000ull) != 0;
  }
  else if (insn->detail->x86.operands[0].size == 8)
  {
    is_zero = result == 0;
    is_sign = (result & 0x8000000000000000ull) != 0;
  }
  else
  {
    assert_always();
  }

  UPDATE_CF(is_carry);
  UPDATE_ZF(is_zero);
  UPDATE_SF(is_sign);
  UPDATE_OF(false);

  thread_context->rip += insn->size;
}
