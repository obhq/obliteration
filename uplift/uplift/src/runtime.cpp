#include "stdafx.h"

#include <queue>

#include <intrin.h>

#include <xenia/base/exception_handler.h>
#include <xenia/base/memory.h>
#include <xenia/base/string.h>
#include <xenia/base/x64_context.h>

#include <capstone/capstone.h>
#include <capstone/x86.h>

#include <xbyak/xbyak.h>

#include "runtime.hpp"
#include "module.hpp"
#include "syscalls.hpp"
#include "bmi1.hpp"

using namespace uplift;

Runtime::Runtime()
  : cpu_()
  , base_path_()
  , object_table_()
  , boot_module_(nullptr)
  , progname_()
  , syscall_table_()
  , entrypoint_(nullptr)
  , fsbase_(nullptr)
  , next_tls_index_(0)
  , user_stack_base_(nullptr)
  , user_stack_end_(nullptr)
  , next_namedobj_id_(0)
  , eport_(nullptr)
{
  get_syscall_table(syscall_table_);
}

Runtime::~Runtime()
{
  if (user_stack_base_ != nullptr)
  {
    xe::memory::DeallocFixed(user_stack_base_, 0, xe::memory::DeallocationType::kRelease);
    user_stack_base_ = nullptr;
  }
}

object_ref<Module> Runtime::FindModuleByName(const std::wstring& path)
{
  auto name = xe::find_name_from_path(path);

  auto modules = object_table_.GetObjectsByType<Module>();
  auto const& it = std::find_if(modules.begin(), modules.end(), [&](const object_ref<Module>& l) { return l->name() == name; });
  if (it == modules.end())
  {
    return nullptr;
  }
  return *it;
}

object_ref<Module> Runtime::LoadModule(const std::wstring& path)
{
  if (!boot_module_)
  {
    return nullptr;
  }

  auto name = xe::find_name_from_path(path);
  auto module = FindModuleByName(name);
  if (module)
  {
    return module;
  }

  module = uplift::Module::Load(this, xe::join_paths(base_path_, path));
  if (!module)
  {
    auto system_path = xe::join_paths(base_path_, L"uplift_sys");
    module = uplift::Module::Load(this, xe::join_paths(system_path, path));
    if (!module)
    {
      return nullptr;
    }
  }
  return module;
}

object_ref<Module> Runtime::LoadExecutable(const std::wstring& path)
{
  auto module = Module::Load(this, xe::join_paths(base_path_, path));
  if (!module)
  {
    object_table_.PurgeAllObjects();
    return nullptr;
  }
  boot_module_ = module.get();

  void* entrypoint;
  if (!module->has_dynamic())
  {
    entrypoint = module->entrypoint();
  }
  else
  {
    auto libkernel = LoadModule(L"libkernel.prx");
    if (!libkernel)
    {
      printf("COULD NOT PRELOAD libkernel!\n");
      object_table_.PurgeAllObjects();
      return nullptr;
    }

    if (!LoadModule(L"libSceLibcInternal.prx"))
    {
      printf("COULD NOT PRELOAD libSceLibcInternal!\n");
      object_table_.PurgeAllObjects();
      return nullptr;
    }

    entrypoint = libkernel->entrypoint();
  }

  entrypoint_ = entrypoint;
  return module;
}

class EntrypointTrampolineGenerator : public Xbyak::CodeGenerator
{
public:
  EntrypointTrampolineGenerator(void* target)
  {
    push(rbp);
    mov(rbp, rsp);
    push(r12); push(r13); push(r14); push(r15);
    push(rdi); push(rsi); push(rbx);

    sub(rsp, 8);

    mov(rdi, rcx);
    mov(rax, (size_t)target);

    call(rax);

    add(rsp, 8);

    pop(rbx); pop(rsi); pop(rdi);
    pop(r15); pop(r14); pop(r13); pop(r12);
    pop(rbp);
    ret();
  }
};

void Runtime::Run(std::vector<std::string>& args)
{
  if (!boot_module_)
  {
    return;
  }

  const size_t user_stack_size = 20 * 1024 * 1024;
  user_stack_base_ = static_cast<uint8_t*>(xe::memory::AllocFixed(
    0, user_stack_size, xe::memory::AllocationType::kReserve, xe::memory::PageAccess::kNoAccess));
  user_stack_end_ = &user_stack_base_[user_stack_size];

  printf("user stack: %p-%p\n", user_stack_base_, user_stack_end_ - 1);

  EntrypointTrampolineGenerator trampoline(entrypoint_);
  auto func = trampoline.getCode<void*(*)(void*)>();

  progname_ = xe::to_string(boot_module_->name());

  union stack_entry
  {
    const void* ptr;
    uint64_t val;
  }
  stack[128];
  stack[0].val = 1 + args.size(); // argc
  auto s = reinterpret_cast<stack_entry*>(&stack[1]);
  (*s++).ptr = progname_.c_str();
  for (auto it = args.begin(); it != args.end(); ++it)
  {
    (*s++).ptr = (*it).c_str();
  }
  (*s++).ptr = nullptr; // arg null terminator
  (*s++).ptr = nullptr; // env null terminator
  (*s++).val = 9ull; // entrypoint type
  (*s++).ptr = boot_module_->entrypoint();
  (*s++).ptr = nullptr; // aux null type
  (*s++).ptr = nullptr;
  
  func(stack);
}

bool Runtime::ResolveSymbol(const Module* skip, uint32_t symbol_name_hash, const std::string& symbol_name, uint64_t& value)
{
  auto modules = object_table_.GetObjectsByType<Module>();
  std::sort(modules.begin(), modules.end(), [](object_ref<Module> a, object_ref<Module> b) { return a->order() < b->order(); });
  for (auto it = modules.begin(); it != modules.end(); ++it)
  {
    if (skip != nullptr && (*it).get() == skip)
    {
      continue;
    }
    if ((*it)->ResolveSymbol(symbol_name_hash, symbol_name, value))
    {
      return true;
    }
  }
  return false;
}

bool Runtime::HandleException(xe::Exception* ex)
{
  if (ex->code() != xe::Exception::Code::kIllegalInstruction)
  {
    return false;
  }

  auto target = reinterpret_cast<uint8_t*>(ex->pc());

  auto instruction_bytes = xe::load_and_swap<uint16_t>(target);
  if (instruction_bytes == 0x0F0B)
  {
    return false;
  }
  else
  {
    auto thread_context = ex->thread_context();
    csh handle;
    if (cs_open(CS_ARCH_X86, CS_MODE_64, &handle) != CS_ERR_OK)
    {
      assert_always();
      return false;
    }
    cs_option(handle, CS_OPT_DETAIL, CS_OPT_ON);
    auto insn = cs_malloc(handle);
    const uint8_t* code = target;
    size_t code_size = 15;
    uint64_t address = ex->pc();
    bool result = false;
    if (cs_disasm_iter(handle, &code, &code_size, &address, insn))
    {
      if (insn->id == X86_INS_ANDN)
      {
        simulate_andn(insn, thread_context);
        result = true;
      }
      else if (insn->id == X86_INS_BEXTR)
      {
        simulate_bextr(insn, thread_context);
        result = true;
      }
      else if (insn->id == X86_INS_BLSI)
      {
        simulate_blsi(insn, thread_context);
        result = true;
      }
      else if (insn->id == X86_INS_BLSR)
      {
        simulate_blsr(insn, thread_context);
        result = true;
      }
    }
    cs_free(insn, 1);
    cs_close(&handle);
    return result;
  }

  return false;
}

bool syscall_dispatch_trampoline(
  Runtime* runtime, uint64_t id,
  uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5, uint64_t arg6,
  SyscallReturnValue& result)
{
  uint64_t args[6];
  args[0] = arg1;
  args[1] = arg2;
  args[2] = arg3;
  args[3] = arg4;
  args[4] = arg5;
  args[5] = arg6;
  return runtime->HandleSyscall(id, result, args);
}

void* Runtime::syscall_handler() const
{
  return syscall_dispatch_trampoline;
}

bool Runtime::HandleSyscall(uint64_t id, SyscallReturnValue& result, uint64_t args[6])
{
  if (id >= _countof(syscall_table_) || syscall_table_[id].handler == nullptr)
  {
    printf("UNKNOWN SYSCALL: %I64u\n", id);
    result.val = -1;
    assert_always();
    return false;
  }
  if (id != 4) printf("SYSCALL(%03I64d): %s\n", id, syscall_table_[id].name);
  return static_cast<SYSCALL_HANDLER>(syscall_table_[id].handler)(this, result, args[0], args[1], args[2], args[3], args[4], args[5]);
}

void Runtime::set_fsbase(void* fsbase)
{
  fsbase_ = fsbase;
  auto modules = object_table_.GetObjectsByType<Module>();
  for (auto it = modules.begin(); it != modules.end(); ++it)
  {
    (*it)->set_fsbase(fsbase);
  }
}

bool Runtime::LoadNeededModules()
{
  printf("LOADING NEEDED MODULES\n");
  auto modules = object_table_.GetObjectsByType<Module>();

  std::queue<object_ref<Module>> queue;
  for (auto it = modules.begin(); it != modules.end(); ++it)
  {
    queue.push(*it);
  }

  while (queue.size() > 0)
  {
    auto module = queue.front();
    queue.pop();

    auto shared_object_names = module->dynamic_info().shared_object_names;
    for (auto it = shared_object_names.begin(); it != shared_object_names.end(); ++it)
    {
      const auto& shared_object_name = *it;

      if (FindModuleByName(shared_object_name))
      {
        continue;
      }

      auto module = LoadModule(shared_object_name);
      if (!module)
      {
        printf("Failed to preload needed '%S'.\n", shared_object_name.c_str());
        continue;
      }

      queue.push(module);
    }
  }

  return true;
}

bool Runtime::SortModules()
{
  std::vector<std::wstring> names;
  std::vector<std::wstring> sorted_names;

  std::queue<Module*> queue;

  uint32_t order = 1;

  auto modules = object_table_.GetObjectsByType<Module>();
  for (auto it = modules.begin(); it != modules.end(); ++it)
  {
    auto module = (*it).get();
    auto name = module->name();

    if (name == L"libkernel.prx" || name == L"libSceLibcInternal.prx")
    {
      sorted_names.push_back(name);
      module->set_order(order++);
      continue;
    }

    queue.push(module);
    names.push_back(module->name());
  }

  while (queue.size() > 0)
  {
    auto module = queue.front();
    queue.pop();

    const auto& shared_object_names = module->dynamic_info().shared_object_names;
    bool requeue = false;
    for (auto it = shared_object_names.begin(); requeue == false && it != shared_object_names.end(); ++it)
    {
      auto const& shared_object_name = *it;
      auto const& it2 = std::find(names.begin(), names.end(), shared_object_name);
      auto const& it3 = std::find(sorted_names.begin(), sorted_names.end(), shared_object_name);
      if (it2 != names.end() && it3 == sorted_names.end())
      {
        requeue = true;
        break;
      }
    }

    if (requeue == true)
    {
      queue.push(module);
      continue;
    }

    module->set_order(order++);
    sorted_names.push_back(module->name());
  }

  return true;
}

bool Runtime::RelocateModules()
{
  printf("RELOCATING MODULES\n");
  auto modules = object_table_.GetObjectsByType<Module>();
  std::sort(modules.begin(), modules.end(), [](object_ref<Module> a, object_ref<Module> b) { return a->order() < b->order(); });
  for (auto it = modules.begin(); it != modules.end(); ++it)
  {
    if (!(*it)->Relocate())
    {
      return false;
    }
  }
  return true;
}
