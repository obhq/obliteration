#pragma once

#include <cstdint>
#include <llvm/BinaryFormat/ELF.h>

namespace uplift
{
  struct ProgramInfo
  {
    llvm::ELF::Elf64_Addr load_start;
    llvm::ELF::Elf64_Addr load_end;

    bool has_dynamic;
    llvm::ELF::Elf64_Half dynamic_index;
    llvm::ELF::Elf64_Addr dynamic_address;
    llvm::ELF::Elf64_Off dynamic_file_offset;
    llvm::ELF::Elf64_Xword dynamic_file_size;
    llvm::ELF::Elf64_Addr tls_address;
    llvm::ELF::Elf64_Xword tls_memory_size;
    llvm::ELF::Elf64_Xword tls_file_size;
    llvm::ELF::Elf64_Xword tls_align;
    llvm::ELF::Elf64_Half sce_dynlibdata_index;
    llvm::ELF::Elf64_Off sce_dynlibdata_file_offset;
    llvm::ELF::Elf64_Xword sce_dynlibdata_file_size;
    llvm::ELF::Elf64_Addr sce_proc_param_address;
    llvm::ELF::Elf64_Xword sce_proc_param_file_size;
    llvm::ELF::Elf64_Addr eh_frame_address;
    llvm::ELF::Elf64_Xword eh_frame_memory_size;
    llvm::ELF::Elf64_Half sce_comment_index;
    llvm::ELF::Elf64_Off sce_comment_file_offset;
    llvm::ELF::Elf64_Xword sce_comment_file_size;
  };

  bool get_program_info(llvm::ELF::Elf64_Phdr* phdr, llvm::ELF::Elf64_Half count, ProgramInfo& info);
}
