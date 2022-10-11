#pragma once

#include <xenia/base/exception_handler.h>

#include <capstone/capstone.h>
#include <capstone/x86.h>

#include <xbyak/xbyak.h>

void simulate_andn(cs_insn* insn, xe::X64Context* thread_context);
void simulate_bextr(cs_insn* insn, xe::X64Context* thread_context);
void simulate_blsi(cs_insn* insn, xe::X64Context* thread_context);
void simulate_blsr(cs_insn* insn, xe::X64Context* thread_context);
