#pragma once

#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

#include <llvm/BinaryFormat/ELF.h>

#include "rip_zone.hpp"
#include "rip_pointers.hpp"

#include "program_info.hpp"
#include "dynamic_info.hpp"

#include "objects/object.hpp"

namespace uplift
{
  class Runtime;

  class Module : public objects::Object
  {
  public:
    static const Object::Type ObjectType = Type::Module;

    static object_ref<Module> Load(Runtime* runtime, const std::wstring& path);

    Module(Runtime* runtime, const std::wstring& path);
    virtual ~Module();

    const std::wstring& name() const { return name_; }

    uint32_t order() const { return order_; }

    uint16_t type() const { return type_; }

    bool has_dynamic() const { return dynamic_buffer_ != nullptr; }

    uint64_t sce_proc_param_address() const { return sce_proc_param_address_; }
    size_t sce_proc_param_size() const { return sce_proc_param_size_; }

    uint8_t* base_address() const { return base_address_; }
    uint8_t* text_address() const { return text_address_; }
    size_t text_size() const { return text_size_; }
    uint8_t* data_address() const { return data_address_; }
    size_t data_size() const { return data_size_; }

    uint8_t* eh_frame_data_buffer() const { return eh_frame_data_buffer_; }
    size_t eh_frame_data_size() const { return static_cast<size_t>(eh_frame_data_buffer_end_ - eh_frame_data_buffer_); }

    void* entrypoint() const { return base_address_ ? &base_address_[entrypoint_] : nullptr; }

    uint16_t tls_index() const { return tls_index_; }

    ProgramInfo program_info() const { return program_info_; }
    DynamicInfo dynamic_info() const { return dynamic_info_; }

    void set_order(uint32_t order) { order_ = order; }
    void set_fsbase(void* fsbase);

    bool ResolveSymbol(uint32_t hash, const std::string& name, uint64_t& value);
    bool Relocate();

    void Protect();
    void Unprotect();

  public:
    SyscallError Close();

    SyscallError Read(void* data_buffer, size_t data_size, size_t* read_size);
    SyscallError Write(const void* data_buffer, size_t data_size, size_t* written_size);
    SyscallError IOControl(uint32_t request, void* argp);
    SyscallError MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation);

  private:
    bool ProcessEHFrame();
    bool ProcessDynamic();
    bool AnalyzeAndPatchCode();

    bool ResolveExternalSymbol(const std::string& local_name, uint64_t& value);

    bool RelocateRela();
    bool RelocatePltRela();

    Runtime* runtime_;

    std::wstring path_;
    std::wstring name_;

    uint32_t order_;

    llvm::ELF::Elf64_Half type_;

    uint8_t* dynamic_buffer_;
    size_t dynamic_size_;
    uint8_t* sce_dynlibdata_buffer_;
    size_t sce_dynlibdata_size_;
    uint8_t* sce_comment_buffer_;
    size_t sce_comment_size_;

    uint8_t* reserved_address_;
    size_t reserved_prefix_size_;
    size_t reserved_suffix_size_;

    uint8_t* base_address_;
    uint8_t* text_address_;
    size_t text_size_;
    uint8_t* data_address_;
    size_t data_size_;

    RIPPointers* rip_pointers_;
    RIPZone rip_zone_;

    uint64_t sce_proc_param_address_;
    uint64_t sce_proc_param_size_;

    uint8_t* eh_frame_data_buffer_;
    uint8_t* eh_frame_data_buffer_end_;

    uint64_t entrypoint_;

    uint16_t tls_index_;

    std::vector<llvm::ELF::Elf64_Phdr> load_headers_;

    std::unordered_map<uint8_t*, uint8_t> interrupts_;

    ProgramInfo program_info_;
    DynamicInfo dynamic_info_;
  };
}
