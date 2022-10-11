#pragma once

#include <memory>
#include <vector>

#include <xenia/base/exception_handler.h>

#include <xbyak/xbyak_util.h>

#include "module.hpp"
#include "object_table.hpp"
#include "syscalls.hpp"

namespace uplift::objects
{
  class Eport;
}

namespace uplift
{
  class Runtime
  {
    friend class SYSCALLS;

  public:
    Runtime();
    virtual ~Runtime();

    bool cpu_has(Xbyak::util::Cpu::Type type)
    {
      return cpu_.has(type);
    };

    ObjectTable* object_table() { return &object_table_; }

    void* fsbase() const { return fsbase_; }
    void* syscall_handler() const;

    void set_base_path(const std::wstring& base_path)
    {
      base_path_ = base_path;
    }

    uint16_t next_tls_index() { assert_true(next_tls_index_ < 0xFFFFu); return next_tls_index_++; }

    object_ref<Module> FindModuleByName(const std::wstring& name);
    object_ref<Module> LoadModule(const std::wstring& path);

    object_ref<Module> LoadExecutable(const std::wstring& path);

    void Run(std::vector<std::string>& args);

    bool ResolveSymbol(const Module* skip, uint32_t symbol_name_hash, const std::string& symbol_name, uint64_t& value);

    bool HandleSyscall(uint64_t id, SyscallReturnValue& result, uint64_t args[6]);
    bool HandleException(xe::Exception* ex);

  private:
    void set_fsbase(void* fsbase);

    bool LoadNeededModules();
    bool SortModules();
    bool RelocateModules();

    Xbyak::util::Cpu cpu_;
    std::wstring base_path_;

    ObjectTable object_table_;
    Module* boot_module_;
    std::vector<Module*> sorted_modules_;

    std::string progname_;

    SyscallEntry syscall_table_[SyscallTableSize];

    void* entrypoint_;
    void* fsbase_;
    uint16_t next_tls_index_;
    uint8_t* user_stack_base_;
    uint8_t* user_stack_end_;
    uint32_t next_namedobj_id_;

    objects::Eport* eport_;
  };
}
