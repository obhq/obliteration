#pragma once

#include <string>
#include <vector>

#include <llvm/BinaryFormat/ELF.h>

namespace uplift
{
  struct ModuleInfo
  {
    std::string name;
    union
    {
      uint64_t value;
      struct
      {
        uint32_t name_offset;
        uint8_t version_minor;
        uint8_t version_major;
        uint16_t id;
      };
    };
    uint16_t attributes;
  };

  struct LibraryInfo
  {
    std::string name;
    union
    {
      uint64_t value;
      struct
      {
        uint32_t name_offset;
        uint16_t version;
        uint16_t id;
      };
    };
    uint16_t attributes;
    bool is_export;
  };

  enum class DynamicFlags : uint32_t
  {
    HasTextRelocations = 1ull << 3,
    IsSymbolic = 1ull << 4,
    BindNow = 1ull << 5,
    NoDelete = 1ull << 11,
    NoOpen = 1ull << 12,
    LoadFilter = 1ull << 13,
  };

  inline uint32_t& operator |=(uint32_t& a, DynamicFlags b)
  {
    return a = a | static_cast<uint32_t>(b);
  }

  inline uint32_t operator &(uint32_t a, DynamicFlags b)
  {
    return a & static_cast<uint32_t>(b);
  }

  struct DynamicInfo
  {
    llvm::ELF::Elf64_Xword rela_table_offset;
    llvm::ELF::Elf64_Xword rela_table_size;
    llvm::ELF::Elf64_Xword pltrela_table_offset;
    llvm::ELF::Elf64_Xword pltrela_table_size;
    llvm::ELF::Elf64_Xword string_table_offset;
    llvm::ELF::Elf64_Xword string_table_size;
    llvm::ELF::Elf64_Xword symbol_table_offset;
    llvm::ELF::Elf64_Xword symbol_table_size;
    llvm::ELF::Elf64_Xword hash_table_offset;
    llvm::ELF::Elf64_Xword hash_table_size;

    uint32_t flags;
    std::vector<std::wstring> shared_object_names;
    std::wstring shared_object_name;
    std::vector<ModuleInfo> modules;
    std::vector<LibraryInfo> libraries;
    llvm::ELF::Elf64_Xword pltgot_offset;
    llvm::ELF::Elf64_Xword init_offset;
    bool has_init_offset;
    llvm::ELF::Elf64_Xword fini_offset;
    bool has_fini_offset;
    uint8_t fingerprint[20];
    std::string output_image_name;

    bool find_module(uint16_t id, ModuleInfo& info);
    bool find_library(uint16_t id, LibraryInfo& info);
  };

  bool get_dynamic_info(llvm::ELF::Elf64_Dyn* entry, size_t entry_count, uint8_t* data_buffer, size_t data_size, DynamicInfo& info);
}
