#include "stdafx.h"

#include <llvm/BinaryFormat/ELF.h>

#include <xenia/base/assert.h>
#include <xenia/base/math.h>

#include "program_info.hpp"
#include "helpers.hpp"

using namespace uplift;
namespace elf = llvm::ELF;

bool uplift::get_program_info(elf::Elf64_Phdr* phdr, elf::Elf64_Half count, ProgramInfo& info)
{
  std::memset(&info, 0, sizeof(info));
  info.load_start = UINT64_MAX;
  info.load_end = 0;

  if (count == 0)
  {
    return false;
  }

  for (elf::Elf64_Half i = 0; i < count; ++i, ++phdr)
  {
    switch (phdr->p_type)
    {
      case elf::PT_LOAD:
      {
        if (phdr->p_align & 0x3FFFull || phdr->p_vaddr & 0x3FFFull || phdr->p_offset & 0x3FFFull)
        {
          return false;
        }

        if (phdr->p_filesz > phdr->p_memsz)
        {
          return false;
        }

        if (info.load_start == UINT64_MAX || phdr->p_vaddr < info.load_start)
        {
          info.load_start = phdr->p_vaddr;
        }

        auto aligned_end = xe::align<elf::Elf64_Addr>(phdr->p_vaddr + phdr->p_memsz, 0x4000);
        if (aligned_end >= info.load_end)
        {
          info.load_end = aligned_end;
        }

        break;
      }

      case elf::PT_DYNAMIC:
      {
        if (phdr->p_filesz > phdr->p_memsz)
        {
          return false;
        }

        info.has_dynamic = true;
        info.dynamic_index = i;
        info.dynamic_address = phdr->p_vaddr;
        info.dynamic_file_offset = phdr->p_offset;
        info.dynamic_file_size = phdr->p_filesz;
        break;
      }

      case elf::PT_TLS:
      {
        if (phdr->p_filesz > phdr->p_memsz)
        {
          return false;
        }

        if (phdr->p_align > 32)
        {
          return false;
        }

        info.tls_address = phdr->p_vaddr;
        info.tls_memory_size = phdr->p_memsz;
        info.tls_file_size = phdr->p_filesz;
        info.tls_align = phdr->p_align;
        break;
      }

      case 0x61000000u:
      {
        if (phdr->p_filesz == 0)
        {
          return false;
        }

        info.sce_dynlibdata_index = i;
        info.sce_dynlibdata_file_offset = phdr->p_offset;
        info.sce_dynlibdata_file_size = phdr->p_filesz;
        break;
      }

      case 0x61000001u:
      {
        info.sce_proc_param_address = phdr->p_vaddr;
        info.sce_proc_param_file_size = phdr->p_filesz;
        break;
      }

      case 0x6474E550u:
      {
        if (phdr->p_filesz > phdr->p_memsz)
        {
          return false;
        }

        info.eh_frame_address = phdr->p_vaddr;
        info.eh_frame_memory_size = phdr->p_memsz;
        break;
      }

      case 0x6FFFFF00u:
      {
        info.sce_comment_index = i;
        info.sce_comment_file_offset = phdr->p_offset;
        info.sce_comment_file_size = phdr->p_filesz;
        break;
      }
    }
  }

  if (info.load_start == UINT64_MAX || info.load_end == 0ull)
  {
    return false;
  }

  return true;
}
