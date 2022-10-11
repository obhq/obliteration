#include "stdafx.h"

#include <algorithm>

#include <xenia/base/assert.h>
#include <xenia/base/debugging.h>
#include <xenia/base/mapped_memory.h>
#include <xenia/base/memory.h>
#include <xenia/base/string.h>

//#define NOMINMAX
//#include <windows.h>

#include <llvm/BinaryFormat/ELF.h>

#include <capstone/capstone.h>
#include <capstone/x86.h>

#include <xbyak/xbyak.h>

#include "runtime.hpp"
#include "module.hpp"
#include "objects/object.hpp"
#include "program_info.hpp"
#include "dynamic_info.hpp"
#include "helpers.hpp"
#include "match.hpp"
#include "code_generators.hpp"
#include "syscall_errors.hpp"

using namespace uplift;
namespace elf = llvm::ELF;

bool is_loadable(elf::Elf64_Word type)
{
  return type == elf::PT_LOAD || type == 0x61000010ull;
}

object_ref<Module> Module::Load(Runtime* runtime, const std::wstring& path)
{
  if (runtime == nullptr)
  {
    return nullptr;
  }

  auto map = xe::MappedMemory::Open(path, xe::MappedMemory::Mode::kRead);
  if (map == nullptr)
  {
    return nullptr;
  }

  auto data = map->data();

  auto ehdr = reinterpret_cast<elf::Elf64_Ehdr*>(data);
  if (!ehdr->checkMagic())
  {
    return nullptr;
  }

  if (ehdr->getFileClass() != elf::ELFCLASS64)
  {
    return nullptr;
  }

  if (ehdr->getDataEncoding() != elf::ELFDATA2LSB)
  {
    return nullptr;
  }

  if (ehdr->e_type != elf::ET_EXEC && ehdr->e_type != 0xFE00u && ehdr->e_type != 0xFE10u && ehdr->e_type != 0xFE18u)
  {
    return nullptr;
  }

  if (ehdr->e_machine != elf::EM_X86_64)
  {
    return nullptr;
  }

  if (ehdr->e_version != llvm::ELF::EV_CURRENT)
  {
    return nullptr;
  }

  ProgramInfo info;
  if (!get_program_info(reinterpret_cast<elf::Elf64_Phdr*>(&ehdr[1]), ehdr->e_phnum, info))
  {
    return nullptr;
  }

  uint64_t text_address = 0ull;
  size_t text_size = 0ull;
  uint64_t data_address = 0ull;
  size_t data_size = 0ull;
  for (elf::Elf64_Half i = 0; i < ehdr->e_phnum; ++i)
  {
    auto phdr = &reinterpret_cast<elf::Elf64_Phdr*>(&ehdr[1])[i];
    if (!is_loadable(phdr->p_type) || !phdr->p_memsz)
    {
      continue;
    }
    if (phdr->p_flags & elf::PF_X)
    {
      if (phdr->p_memsz > text_size)
      {
        text_address = phdr->p_vaddr;
        text_size = phdr->p_memsz;
      }
    }
    else
    {
      if (phdr->p_memsz > data_size)
      {
        data_address = phdr->p_vaddr;
        data_size = phdr->p_memsz;
      }
    }
  }

  if (!info.has_dynamic && ehdr->e_type == 0xFE10u)
  {
    return nullptr;
  }

  bool had_error = false;
  uint8_t* dynamic_buffer = nullptr;
  uint8_t* sce_dynlibdata_buffer = nullptr;

  if (info.has_dynamic)
  {
    if (!info.dynamic_file_size || !info.sce_dynlibdata_file_size)
    {
      goto error;
    }

    dynamic_buffer = static_cast<uint8_t*>(xe::memory::AllocFixed(
      nullptr,
      info.dynamic_file_size,
      xe::memory::AllocationType::kReserveCommit,
      xe::memory::PageAccess::kReadWrite));
    if (dynamic_buffer == nullptr)
    {
      goto error;
    }

    std::memcpy(dynamic_buffer, &data[info.dynamic_file_offset], info.dynamic_file_size);

    sce_dynlibdata_buffer = static_cast<uint8_t*>(xe::memory::AllocFixed(
      nullptr,
      info.sce_dynlibdata_file_size,
      xe::memory::AllocationType::kReserveCommit,
      xe::memory::PageAccess::kReadWrite));
    if (sce_dynlibdata_buffer == nullptr)
    {
      goto error;
    }

    std::memcpy(sce_dynlibdata_buffer, &data[info.sce_dynlibdata_file_offset], info.sce_dynlibdata_file_size);
  }

  auto load_size = info.load_end - info.load_start;

  union address
  {
    uint8_t* ptr;
    uint64_t val;
  };

  const size_t one_mb = 1ull * 1024ull * 1024ull;
  const size_t eight_mb = 8ull * one_mb;
  const size_t thirtytwo_mb = 32ull * one_mb;
  const size_t four_gb = 4ull * 1024ull * one_mb;
  const size_t eight_gb = 8ull * 1024ull * one_mb;

  /* 8GiB is reserved so that the loaded module has a guaranteed
   * 4GB address space all to itself, and then some.
   *
   * xxxxxxxx00000000 - xxxxxxxxFFFFFFFF
   *
   * Once the range is mapped, a safe area before or after the
   * size of the loaded module is chosen to store any extra
   * code or data that is easily RIP addressable.
   */

  address reserved_address;
  reserved_address.ptr = static_cast<uint8_t*>(xe::memory::AllocFixed(
    nullptr,
    eight_gb,
    xe::memory::AllocationType::kReserve,
    xe::memory::PageAccess::kNoAccess));
  if (reserved_address.ptr == nullptr)
  {
    goto error;
  }

  const size_t page_size = 0x4000;

  struct rip_zone_definition
  {
    RIPPointers rip_pointers;
    uint8_t padding1[xe::align_const<size_t>(sizeof(RIPPointers), page_size) - sizeof(RIPPointers)];
    uint8_t safe1[page_size];
    uint8_t free_zone[thirtytwo_mb];
    //uint8_t padding2[align_size_const(eight_mb, page_size) - eight_mb];
    uint8_t safe2[page_size];
  };

  const size_t desired_rip_zone_size = sizeof(rip_zone_definition);

  address base_address;
  base_address.val = xe::align<uint64_t>(reserved_address.val, four_gb);

  address reserved_address_aligned;
  reserved_address_aligned.val = xe::align<uint64_t>(reserved_address.val, page_size);

  auto reserved_before_size = static_cast<size_t>(base_address.ptr - reserved_address_aligned.ptr);
  auto reserved_after_size = static_cast<size_t>(&reserved_address.ptr[eight_gb] - &base_address.ptr[load_size]);

  address rip_zone_start, rip_zone_end;

  if (reserved_before_size >= desired_rip_zone_size)
  {
    rip_zone_start.val = reserved_before_size + load_size < INT32_MAX
      ? (reserved_address_aligned.val) & ~(page_size - 1)
      : (base_address.val + load_size + INT32_MIN) & ~(page_size - 1);
    rip_zone_end.ptr = &rip_zone_start.ptr[desired_rip_zone_size];
    assert_true(rip_zone_start.ptr >= reserved_address_aligned.ptr && rip_zone_end.ptr <= base_address.ptr);
  }
  else if (reserved_after_size >= desired_rip_zone_size)
  {
    rip_zone_start.val = xe::align<uint64_t>(base_address.val + load_size, page_size);
    rip_zone_end.ptr = &rip_zone_start.ptr[desired_rip_zone_size];
    assert_true(rip_zone_start.ptr >= &base_address.ptr[load_size] && rip_zone_end.ptr <= &reserved_address.ptr[eight_gb]);
  }
  else
  {
    assert_always();
  }

  auto rip_zone_data = reinterpret_cast<rip_zone_definition*>(rip_zone_start.ptr);

  auto rip_pointers = &rip_zone_data->rip_pointers;
  if (xe::memory::AllocFixed(
    rip_pointers,
    sizeof(RIPPointers),
    xe::memory::AllocationType::kCommit,
    xe::memory::PageAccess::kReadWrite) != rip_pointers)
  {
    goto error;
  }

  rip_pointers->runtime = runtime;
  rip_pointers->syscall_handler = runtime->syscall_handler();

  auto free_zone = &rip_zone_data->free_zone[0];
  if (xe::memory::AllocFixed(
    free_zone,
    sizeof(rip_zone_definition::free_zone),
    xe::memory::AllocationType::kCommit,
    xe::memory::PageAccess::kExecuteReadWrite) != free_zone)
  {
    goto error;
  }

  for (elf::Elf64_Half i = 0; i < ehdr->e_phnum; ++i)
  {
    auto phdr = &reinterpret_cast<elf::Elf64_Phdr*>(&ehdr[1])[i];
    if (!is_loadable(phdr->p_type) || phdr->p_memsz == 0)
    {
      continue;
    }

    auto program_address = &base_address.ptr[phdr->p_vaddr];
    auto program_allocated_address = xe::memory::AllocFixed(
      program_address,
      phdr->p_memsz,
      xe::memory::AllocationType::kCommit,
      xe::memory::PageAccess::kReadWrite);
    if (program_allocated_address == nullptr || program_allocated_address != program_address)
    {
      goto error;
    }

    std::memcpy(program_address, &data[phdr->p_offset], phdr->p_filesz);

    if (phdr->p_memsz > phdr->p_filesz)
    {
      std::memset(&program_address[phdr->p_filesz], 0, phdr->p_memsz - phdr->p_filesz);
    }
  }

  // we're good
  {
    auto module = object_ref<Module>(new Module(runtime, path));
    module->type_ = ehdr->e_type;
    module->dynamic_buffer_ = dynamic_buffer;
    module->dynamic_size_ = info.dynamic_file_size;
    module->sce_dynlibdata_buffer_ = sce_dynlibdata_buffer;
    module->sce_dynlibdata_size_ = info.sce_dynlibdata_file_size;
    module->reserved_address_ = reserved_address.ptr;
    module->reserved_prefix_size_ = reserved_before_size;
    module->reserved_suffix_size_ = reserved_after_size;
    module->base_address_ = base_address.ptr;
    module->text_address_ = &base_address.ptr[text_address];
    module->text_size_ = text_size;
    module->data_address_ = &base_address.ptr[data_address];
    module->data_size_ = data_size;
    module->rip_pointers_ = rip_pointers;
    module->rip_zone_ =
    {
      &free_zone[0],
      &free_zone[0],
      &free_zone[sizeof(rip_zone_definition::free_zone)],
    };
    module->sce_proc_param_address_ = info.sce_proc_param_address;
    module->sce_proc_param_size_ = info.sce_proc_param_file_size;
    module->entrypoint_ = ehdr->e_entry;
    for (elf::Elf64_Half i = 0; i < ehdr->e_phnum; ++i)
    {
      auto phdr = &reinterpret_cast<elf::Elf64_Phdr*>(&ehdr[1])[i];
      if (!is_loadable(phdr->p_type) || phdr->p_memsz == 0)
      {
        continue;
      }
      module->load_headers_.push_back(*phdr);
    }
    module->program_info_ = info;
    if (!module->ProcessEHFrame())
    {
      assert_always();
    }
    if (!module->ProcessDynamic())
    {
      assert_always();
    }
    if (!module->AnalyzeAndPatchCode())
    {
      assert_always();
    }
    module->Protect();

    xe::debugging::DebugPrint(
      "LOAD MODULE: %S @ %p (%p, %p)\n",
      module->name().c_str(),
      base_address.ptr,
      !module->dynamic_info().has_init_offset ? nullptr : &base_address.ptr[module->dynamic_info().init_offset],
      !module->dynamic_info().has_fini_offset ? nullptr : &base_address.ptr[module->dynamic_info().fini_offset]);

    return module;
  }

error:
  if (free_zone)
  {
    xe::memory::DeallocFixed(free_zone, 0, xe::memory::DeallocationType::kRelease);
  }
  if (rip_pointers)
  {
    xe::memory::DeallocFixed(rip_pointers, 0, xe::memory::DeallocationType::kRelease);
  }
  if (reserved_address.ptr)
  {
    xe::memory::DeallocFixed(reserved_address.ptr, 0, xe::memory::DeallocationType::kRelease);
  }
  if (sce_dynlibdata_buffer)
  {
    xe::memory::DeallocFixed(sce_dynlibdata_buffer, 0, xe::memory::DeallocationType::kRelease);
  }
  if (dynamic_buffer)
  {
    xe::memory::DeallocFixed(dynamic_buffer, 0, xe::memory::DeallocationType::kRelease);
  }
  return nullptr;
}

Module::Module(Runtime* runtime, const std::wstring& path)
  : Object(runtime, ObjectType)
  , runtime_(runtime)
  , path_(path)
  , name_(xe::find_name_from_path(path))
  , order_(0)
  , type_(0)
  , dynamic_buffer_(nullptr)
  , dynamic_size_(0)
  , sce_dynlibdata_buffer_(nullptr)
  , sce_dynlibdata_size_(0)
  , sce_comment_buffer_(nullptr)
  , sce_comment_size_(0)
  , reserved_address_(nullptr)
  , reserved_prefix_size_(0)
  , reserved_suffix_size_(0)
  , base_address_(nullptr)
  , rip_pointers_(nullptr)
  , rip_zone_()
  , sce_proc_param_address_(0)
  , sce_proc_param_size_(0)
  , eh_frame_data_buffer_(nullptr)
  , eh_frame_data_buffer_end_(nullptr)
  , entrypoint_(0)
  , tls_index_(runtime->next_tls_index())
  , program_info_()
  , dynamic_info_()
{
}

Module::~Module()
{
  if (rip_pointers_)
  {
    xe::memory::DeallocFixed(rip_pointers_, 0, xe::memory::DeallocationType::kRelease);
    rip_pointers_ = nullptr;
  }
  if (reserved_address_)
  {
    xe::memory::DeallocFixed(reserved_address_, 0, xe::memory::DeallocationType::kRelease);
    reserved_address_ = nullptr;
  }
  if (sce_dynlibdata_buffer_)
  {
    xe::memory::DeallocFixed(sce_dynlibdata_buffer_, 0, xe::memory::DeallocationType::kRelease);
    sce_dynlibdata_buffer_ = nullptr;
  }
  if (dynamic_buffer_)
  {
    xe::memory::DeallocFixed(dynamic_buffer_, 0, xe::memory::DeallocationType::kRelease);
    dynamic_buffer_ = nullptr;
  }
}

bool Module::ProcessEHFrame()
{
  if (program_info_.eh_frame_address == 0 || program_info_.eh_frame_memory_size < 4)
  {
    return false;
  }

  uint8_t* current;

  auto header_buffer = &base_address_[program_info_.eh_frame_address];

  auto version = header_buffer[0];
  auto data_pointer_encoding = header_buffer[1];
  auto fde_count_encoding = header_buffer[2];
  auto search_table_pointer_encoding = header_buffer[3];
  current = &header_buffer[4];

  if (version != 1)
  {
    return false;
  }

  uint8_t* data_buffer;
  if (data_pointer_encoding == 0x03) // relative to base address
  {
    auto offset = *reinterpret_cast<uint32_t*>(current);
    current += 4;
    data_buffer = &base_address_[offset];
  }
  else if (data_pointer_encoding == 0x1B) // relative to eh_frame
  {
    auto offset = *reinterpret_cast<int32_t*>(current);
    current += 4;
    data_buffer = &current[offset];
  }
  else
  {
    return false;
  }

  if (!data_buffer)
  {
    return false;
  }

  uint8_t* data_buffer_end = data_buffer;
  while (true)
  {
    size_t size = *reinterpret_cast<int32_t*>(data_buffer_end);
    if (size == 0)
    {
      data_buffer_end = &data_buffer_end[4];
      break;
    }
    if (size == -1)
    {
      size = 12 + *reinterpret_cast<size_t*>(&data_buffer_end[4]);
    }
    else
    {
      size = 4 + size;
    }
    data_buffer_end = &data_buffer_end[size];
  }

  size_t fde_count;
  if (fde_count_encoding == 0x03) // absolute
  {
    fde_count = *reinterpret_cast<uint32_t*>(current);
    current += 4;
  }
  else
  {
    return false;
  }

  if (search_table_pointer_encoding != 0x3B) // relative to eh_frame
  {
    return false;
  }

  /*
  struct search_table_entry
  {
    int32_t initial_offset;
    int32_t fde_offset;
  };

  auto search_table = reinterpret_cast<const search_table_entry*>(current);
  for (size_t i = 0; i < fde_count; ++i)
  {
    auto search_entry = &search_table[i];

    auto base_code = &header_buffer[search_entry->initial_offset];
    auto fde = &header_buffer[search_entry->fde_offset];

    printf("%p %p\n", base_code, fde);
  }
  */

  eh_frame_data_buffer_ = data_buffer;
  eh_frame_data_buffer_end_ = data_buffer_end;
  return true;
}

bool Module::ProcessDynamic()
{
  return get_dynamic_info(
    reinterpret_cast<elf::Elf64_Dyn*>(dynamic_buffer_),
    dynamic_size_ / sizeof(elf::Elf64_Dyn),
    sce_dynlibdata_buffer_,
    sce_dynlibdata_size_,
    dynamic_info_);
}

bool patch_fsbase_access(uint8_t* target, cs_insn* insn, RIPPointers* rip_pointers, RIPZone& rip_zone)
{
  if (insn->id == X86_INS_MOV)
  {
    if (insn->detail->x86.op_count != 2)
    {
      assert_always();
      return false;
    }

    auto operands = insn->detail->x86.operands;

    if (operands[0].type != X86_OP_REG)
    {
      assert_always();
      return false;
    }

    if (operands[1].type != X86_OP_MEM ||
        operands[1].mem.segment != X86_REG_FS ||
        operands[1].mem.base != X86_REG_INVALID ||
        operands[1].mem.index != X86_REG_INVALID)
    {
      assert_always();
      return false;
    }

    using Generator = code_generators::FSBaseMovGenerator;

    Generator generator(operands[0].reg, operands[0].size, operands[1].mem.disp);

    auto trampoline_code = generator.getCode();
    auto trampoline_size = generator.getSize();
    auto aligned_size = xe::align<size_t>(trampoline_size, 32);

    uint8_t* rip_code;
    if (!rip_zone.take(aligned_size, rip_code))
    {
      assert_always();
      return false;
    }

    std::memcpy(rip_code, trampoline_code, trampoline_size);

    auto tail = reinterpret_cast<Generator::Tail*>(&rip_code[trampoline_size - sizeof(Generator::Tail)]);
    tail->target = &target[insn->size];
    tail->rip_pointers = rip_pointers;

    if (trampoline_size < aligned_size)
    {
      std::memset(&rip_code[trampoline_size], 0xCC, aligned_size - trampoline_size);
    }

    assert_true(insn->size >= 5);

    auto disp = static_cast<uint32_t>(rip_code - &target[5]);
    target[0] = 0xE9;
    *reinterpret_cast<uint32_t*>(&target[1]) = disp;

    if (5 < insn->size)
    {
      std::memset(&target[5], 0xCC, insn->size - 5);
    }

    return true;
  }
  else
  {
    assert_always();
    return false;
  }
}

bool hook_syscall(uint64_t id, uint8_t* target, size_t target_size, RIPPointers* rip_pointers, RIPZone& rip_zone)
{
  if (id == UINT64_MAX)
  {
    using Generator = code_generators::SyscallTrampolineGenerator;
    Generator generator;

    auto trampoline_code = generator.getCode();
    auto trampoline_size = generator.getSize();
    auto aligned_size = xe::align<size_t>(trampoline_size, 32);

    uint8_t* rip_code;
    if (!rip_zone.take(aligned_size, rip_code))
    {
      assert_always();
      return false;
    }

    std::memcpy(rip_code, trampoline_code, trampoline_size);

    auto tail = reinterpret_cast<Generator::Tail*>(&rip_code[trampoline_size - sizeof(Generator::Tail)]);
    tail->target = &target[target_size];
    tail->rip_pointers = rip_pointers;

    if (trampoline_size < aligned_size)
    {
      std::memset(&rip_code[trampoline_size], 0xCC, aligned_size - trampoline_size);
    }

    auto disp = static_cast<uint32_t>(rip_code - &target[5]);
    target[0] = 0xE9;
    *reinterpret_cast<uint32_t*>(&target[1]) = disp;
    return true;
  }
  else
  {
    using Generator = code_generators::NakedSyscallTrampolineGenerator;
    Generator generator(id);

    auto trampoline_code = generator.getCode();
    auto trampoline_size = generator.getSize();
    auto aligned_size = xe::align<size_t>(trampoline_size, 32);

    uint8_t* rip_code;
    if (!rip_zone.take(aligned_size, rip_code))
    {
      assert_always();
      return false;
    }

    std::memcpy(rip_code, trampoline_code, trampoline_size);

    auto tail = reinterpret_cast<Generator::Tail*>(&rip_code[trampoline_size - sizeof(Generator::Tail)]);
    tail->target = &target[target_size];
    tail->rip_pointers = rip_pointers;

    if (trampoline_size < aligned_size)
    {
      std::memset(&rip_code[trampoline_size], 0xCC, aligned_size - trampoline_size);
    }

    auto disp = static_cast<uint32_t>(rip_code - &target[5]);
    target[0] = 0xE9;
    *reinterpret_cast<uint32_t*>(&target[1]) = disp;
    target[5] = 0xCC;
    target[6] = 0xCC;
    target[7] = 0xCC;
    target[8] = 0xCC;
    return true;
  }
  assert_always();
  return false;
}

bool hook_bmi1_instruction(uint8_t* target, cs_insn* insn, RIPPointers* rip_pointers, RIPZone& rip_zone)
{
  //assert_always();
  return true;
}

bool is_bmi1_instruction(x86_insn op)
{
  return op == X86_INS_ANDN ||
    op == X86_INS_BEXTR ||
    op == X86_INS_BLSI ||
    op == X86_INS_BLSMSK ||
    op == X86_INS_BLSR ||
    op == X86_INS_TZCNT;
}

bool Module::AnalyzeAndPatchCode()
{
  if (name_ == L"libkernel.prx")
  {
    // nasty hack to enable libkernel debug messages, only valid for 3.55
    *reinterpret_cast<uint32_t*>(&base_address_[0x6036C]) = 0xFFFFFFFF;
  }

  elf::Elf64_Phdr phdr;
  bool found_code = false;
  for (auto it = load_headers_.begin(); it != load_headers_.end(); ++it)
  {
    phdr = *it;
    if (phdr.p_flags & elf::PF_X)
    {
      found_code = true;
      break;
    }
  }

  if (!found_code)
  {
    return true;
  }

  auto program_buffer = &base_address_[phdr.p_vaddr];
  uint8_t* text_buffer;
  size_t text_size;
  /* This is necessary since both R+X and R sections get merged
   * together in the resulting ELF file. Capstone really doesn't
   * like it when you throw data mixed in, so text_address_ and
   * text_size_ are unreliable. This does signature matching to
   * figure out where the code starts and ends.
   */
  if (!get_text_region(program_buffer, phdr.p_filesz, text_buffer, text_size))
  {
    assert_always();
    return false;
  }

  csh handle;
  if (cs_open(CS_ARCH_X86, CS_MODE_64, &handle) != CS_ERR_OK)
  {
    assert_always();
    return false;
  }
  cs_option(handle, CS_OPT_DETAIL, CS_OPT_ON);

  auto insn = cs_malloc(handle);
  const uint8_t* code = text_buffer;
  size_t code_size = text_size;
  uint64_t address = phdr.p_vaddr + (text_buffer - program_buffer);
  while (cs_disasm_iter(handle, &code, &code_size, &address, insn))
  {
    if (insn->id == X86_INS_SYSCALL)
    {
      assert_true(insn->size == 2);
      auto target = &program_buffer[insn->address];

      uint16_t syscall_pattern[] = { 0x49, 0x89, 0xCA, 0x0F, 0x05 };
      uint16_t naked_syscall_pattern[] = { 0x48, 0xC7, 0xC0, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY, 0x0F, 0x05 };

#define IS_SYSCALL_MATCH(x) \
  (match_buffer(&target[-(_countof(x) - 2)], _countof(x), x, _countof(x), &match) && &target[-(_countof(x) - 2)] == match)
      void* match;
#pragma warning(suppress: 4146)
      if (IS_SYSCALL_MATCH(syscall_pattern))
      {
        hook_syscall(UINT64_MAX, &target[-3], _countof(syscall_pattern), rip_pointers_, rip_zone_);
      }
#pragma warning(suppress: 4146)
      else if (IS_SYSCALL_MATCH(naked_syscall_pattern))
      {
        auto syscall_id = *reinterpret_cast<uint32_t*>(&target[-4]);
        hook_syscall(syscall_id, &target[-7], _countof(naked_syscall_pattern), rip_pointers_, rip_zone_);
      }
      else
      {
        assert_always();
      }
#undef IS_SYSCALL_MATCH
    }
    else if (insn->id == X86_INS_INT)
    {
      assert_true(insn->size == 2);
      auto target = &program_buffer[insn->address];
      target[0] = 0x0F;
      target[1] = 0x0B;
      interrupts_[target] = (uint8_t)insn->detail->x86.operands[0].imm;
    }
    else if (insn->id == X86_INS_INT1)
    {
      assert_unhandled_case(X86_INS_INTO);
    }
    else if (insn->id == X86_INS_INTO)
    {
      assert_unhandled_case(X86_INS_INTO);
    }
    else if (!runtime_->cpu_has(Xbyak::util::Cpu::tBMI1) && is_bmi1_instruction((x86_insn)insn->id))
    {
      assert_true(insn->size >= 5);
      auto target = &program_buffer[insn->address];
      if (!hook_bmi1_instruction(target, insn, rip_pointers_, rip_zone_))
      {
        assert_always();
      }
    }
    else
    {
      bool is_fs = false;
      for (uint8_t i = 0; i < insn->detail->x86.op_count; i++)
      {
        auto operand = insn->detail->x86.operands[i];
        if (operand.type == X86_OP_MEM)
        {
          if (operand.mem.segment == X86_REG_FS)
          {
            is_fs = true;
            break;
          }
          else if (operand.mem.segment == X86_REG_DS ||
                   operand.mem.segment == X86_REG_ES ||
                   operand.mem.segment == X86_REG_GS)
          {
            assert_always();
          }
        }
      }

      if (is_fs)
      {
        auto target = &program_buffer[insn->address];
        patch_fsbase_access(target, insn, rip_pointers_, rip_zone_);
      }
    }
  }
  cs_free(insn, 1);
  cs_close(&handle);
  return true;
}

struct StringTable
{
  const char* buffer;
  size_t length;

  const char* get(size_t offset)
  {
    return offset < length ? &buffer[offset] : nullptr;
  }
};

void Module::set_fsbase(void* fsbase)
{
  if (!rip_pointers_)
  {
    return;
  }

  rip_pointers_->fsbase = fsbase;
}

bool Module::ResolveSymbol(uint32_t symbol_name_hash, const std::string& symbol_name, uint64_t& value)
{
  auto hash_table = reinterpret_cast<elf::Elf64_Word*>(&sce_dynlibdata_buffer_[dynamic_info_.hash_table_offset]);
  auto bucket_count = hash_table[0];
  auto chain_count = hash_table[1];
  auto buckets = &hash_table[2];
  auto chains = &buckets[bucket_count];

  auto symbols = reinterpret_cast<elf::Elf64_Sym*>(&sce_dynlibdata_buffer_[dynamic_info_.symbol_table_offset]);
  auto symbol_count = dynamic_info_.symbol_table_size / sizeof(elf::Elf64_Sym);
  auto symbol_end = &symbols[symbol_count];

  StringTable string_table =
  {
    reinterpret_cast<const char*>(&sce_dynlibdata_buffer_[dynamic_info_.string_table_offset]),
    dynamic_info_.string_table_size,
  };

  for (elf::Elf64_Word index = buckets[symbol_name_hash % bucket_count]; index != elf::STN_UNDEF; index = chains[index])
  {
    if (index >= chain_count)
    {
      return false;
    }
    assert_true(index < symbol_count);
    auto candidate_symbol = symbols[index];
    auto candidate_local_name = string_table.get(candidate_symbol.st_name);
    std::string candidate_symbol_name;
    uint16_t candidate_module_id, candidate_library_id;
    if (parse_symbol_name(candidate_local_name, candidate_symbol_name, candidate_library_id, candidate_module_id))
    {
      ModuleInfo candidate_module;
      LibraryInfo candidate_library;
      if (dynamic_info_.find_module(candidate_module_id, candidate_module) &&
          dynamic_info_.find_library(candidate_library_id, candidate_library))
      {
        if (!candidate_library.is_export)
        {
          continue;
        }

        auto candidate_name = candidate_symbol_name + "#" + candidate_library.name + "#" + candidate_module.name;
        if (candidate_name == symbol_name)
        {
          value = reinterpret_cast<uint64_t>(&base_address_[candidate_symbol.st_value]);
          return true;
        }
      }
    }
  }
  return false;
}

bool Module::ResolveExternalSymbol(const std::string& local_name, uint64_t& value)
{
  std::string symbol_name;
  uint16_t module_id, library_id;
  if (!parse_symbol_name(local_name, symbol_name, library_id, module_id))
  {
    assert_always();
    return false;
  }

  ModuleInfo module;
  LibraryInfo library;
  if (!dynamic_info_.find_module(module_id, module) ||
      !dynamic_info_.find_library(library_id, library))
  {
    assert_always();
    return false;
  }

  auto name = symbol_name + "#" + library.name + "#" + module.name;
  auto name_hash = elf_hash(name.c_str());

  bool is_symbolic = dynamic_info_.flags & DynamicFlags::IsSymbolic;

  if (is_symbolic)
  {
    if (ResolveSymbol(name_hash, name, value))
    {
      return true;
    }
  }

  if (!runtime_->ResolveSymbol(is_symbolic ? this : nullptr, name_hash, name, value))
  {
    printf("FAILED TO RESOLVE: %s\n", name.c_str());

    name = "M0z6Dr6TNnM#libkernel#libkernel"; // sceKernelReportUnpatchedFunctionCall
    name_hash = elf_hash(name.c_str());
    if (!runtime_->ResolveSymbol(is_symbolic ? this : nullptr, name_hash, name, value))
    {
      assert_always();
      return false;
    }
  }
  return true;
}

bool Module::Relocate()
{
  xe::debugging::DebugPrint("RELOCATE MODULE: %S @ %p\n", name_.c_str(), base_address_);

  Unprotect();
  auto result = RelocateRela() && RelocatePltRela();
  Protect();
  return result;
}

bool Module::RelocateRela()
{
  StringTable string_table =
  {
    reinterpret_cast<const char*>(&sce_dynlibdata_buffer_[dynamic_info_.string_table_offset]),
    dynamic_info_.string_table_size,
  };
  auto symbols = reinterpret_cast<elf::Elf64_Sym*>(&sce_dynlibdata_buffer_[dynamic_info_.symbol_table_offset]);
  auto symbol_end = &symbols[dynamic_info_.symbol_table_size / sizeof(elf::Elf64_Sym)];
  auto rela = reinterpret_cast<elf::Elf64_Rela*>(&sce_dynlibdata_buffer_[dynamic_info_.rela_table_offset]);
  auto rela_end = &rela[dynamic_info_.rela_table_size / sizeof(elf::Elf64_Rela)];
  for (; rela < rela_end; ++rela)
  {
    auto type = rela->getType();
    uint64_t symval;
    switch (type)
    {
      case elf::R_X86_64_64:
      case elf::R_X86_64_PC32:
      case elf::R_X86_64_GLOB_DAT:
      case elf::R_X86_64_TPOFF64:
      case elf::R_X86_64_TPOFF32:
      case elf::R_X86_64_DTPMOD64:
      case elf::R_X86_64_DTPOFF64:
      case elf::R_X86_64_DTPOFF32:
      {
        auto symbol = symbols[rela->getSymbol()];
        if (symbol.getBinding() == elf::STB_LOCAL)
        {
          symval = reinterpret_cast<uint64_t>(base_address_) + symbol.st_value;
        }
        else if (symbol.getBinding() == elf::STB_GLOBAL || symbol.getBinding() == elf::STB_WEAK)
        {
          auto local_name = string_table.get(symbol.st_name);
          if (!this->ResolveExternalSymbol(local_name, symval))
          {
            assert_always();
            return false;
          }
        }
        else
        {
          assert_always();
          return false;
        }
        break;
      }

      case elf::R_X86_64_RELATIVE:
      {
        symval = 0;
        break;
      }

      default:
      {
        assert_always();
        return false;
      }
    }

    auto target = &base_address_[rela->r_offset];
    switch (type)
    {
      case elf::R_X86_64_NONE:
      {
        break;
      }

      case elf::R_X86_64_64:
      {
        *reinterpret_cast<uint64_t*>(target) = symval + rela->r_addend;
        break;
      }

      case elf::R_X86_64_PC32:
      {
        auto value = static_cast<uint32_t>(symval + rela->r_addend - reinterpret_cast<uint64_t>(target));
        *reinterpret_cast<uint32_t*>(target) = value;
        break;
      }

      case elf::R_X86_64_COPY:
      {
        assert_always();
        return false;
      }

      case elf::R_X86_64_GLOB_DAT:
      {
        *reinterpret_cast<uint64_t*>(target) = symval;
        break;
      }

      case elf::R_X86_64_TPOFF64:
      {
        assert_always();
        return false;
      }

      case elf::R_X86_64_TPOFF32:
      {
        assert_always();
        return false;
      }

      case elf::R_X86_64_DTPMOD64:
      {
        *reinterpret_cast<uint64_t*>(target) += tls_index_;
        break;
      }

      case elf::R_X86_64_DTPOFF64:
      {
        *reinterpret_cast<uint64_t*>(target) += symval + rela->r_addend;
        break;
      }

      case elf::R_X86_64_DTPOFF32:
      {
        *reinterpret_cast<uint32_t*>(target) += static_cast<uint32_t>(symval + rela->r_addend);
        break;
      }

      case elf::R_X86_64_RELATIVE:
      {
        *reinterpret_cast<uint64_t*>(target) = reinterpret_cast<uint64_t>(base_address_) + rela->r_addend;
        break;
      }

      default:
      {
        assert_always();
        return false;
      }
    }
  }
  return true;
}

bool Module::RelocatePltRela()
{
  StringTable string_table =
  {
    reinterpret_cast<const char*>(&sce_dynlibdata_buffer_[dynamic_info_.string_table_offset]),
    dynamic_info_.string_table_size,
  };
  auto symbols = reinterpret_cast<elf::Elf64_Sym*>(&sce_dynlibdata_buffer_[dynamic_info_.symbol_table_offset]);
  auto symbol_end = &symbols[dynamic_info_.symbol_table_size / sizeof(elf::Elf64_Sym)];
  auto rela = reinterpret_cast<elf::Elf64_Rela*>(&sce_dynlibdata_buffer_[dynamic_info_.pltrela_table_offset]);
  auto rela_end = &rela[dynamic_info_.pltrela_table_size / sizeof(elf::Elf64_Rela)];
  for (; rela < rela_end; ++rela)
  {
    auto type = rela->getType();
    uint64_t symval;
    switch (type)
    {
      case elf::R_X86_64_JUMP_SLOT:
      {
        auto symbol = symbols[rela->getSymbol()];
        if (symbol.getBinding() == elf::STB_LOCAL)
        {
          symval = reinterpret_cast<uint64_t>(base_address_) + symbol.st_value;
        }
        else if (symbol.getBinding() == elf::STB_GLOBAL || symbol.getBinding() == elf::STB_WEAK)
        {
          auto local_name = string_table.get(symbol.st_name);
          if (!this->ResolveExternalSymbol(local_name, symval))
          {
            assert_always();
            return false;
          }
        }
        else
        {
          assert_always();
          return false;
        }
        break;
      }

      case elf::R_X86_64_RELATIVE:
      {
        symval = 0;
        break;
      }

      default:
      {
        assert_always();
        return false;
      }
    }

    auto target = &base_address_[rela->r_offset];
    switch (type)
    {
      case elf::R_X86_64_JUMP_SLOT:
      {
        *reinterpret_cast<uint64_t*>(target) = symval;
        break;
      }

      default:
      {
        assert_always();
        return false;
      }
    }
  }
  return true;
}

void Module::Protect()
{
  for (auto it = load_headers_.begin(); it != load_headers_.end(); ++it)
  {
    auto phdr = *it;
    auto program_address = &base_address_[phdr.p_vaddr];
    xe::memory::Protect(program_address, phdr.p_memsz, get_page_access(phdr.p_flags), nullptr);
  }
}

void Module::Unprotect()
{
  for (auto it = load_headers_.begin(); it != load_headers_.end(); ++it)
  {
    auto phdr = *it;
    auto program_address = &base_address_[phdr.p_vaddr];
    xe::memory::Protect(program_address, phdr.p_memsz, xe::memory::PageAccess::kReadWrite, nullptr);
  }
}

SyscallError Module::Close()
{
  return SyscallError::SUCCESS;
}

SyscallError Module::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SyscallError::eNOSYS;
}

SyscallError Module::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SyscallError::eNOSYS;
}

SyscallError Module::IOControl(uint32_t request, void* argp)
{
  assert_always();
  return SyscallError::eNOSYS;
}

SyscallError Module::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SyscallError::eNOSYS;
}
