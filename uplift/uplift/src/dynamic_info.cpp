#include "stdafx.h"

#include <algorithm>

#include <llvm/BinaryFormat/ELF.h>

#include <xenia/base/assert.h>
#include <xenia/base/string.h>

#include "dynamic_info.hpp"

using namespace uplift;
namespace elf = llvm::ELF;

bool DynamicInfo::find_module(uint16_t id, ModuleInfo& info)
{
  auto it = std::find_if(modules.begin(), modules.end(), [&id](ModuleInfo& m) { return m.id == id; });
  if (it == modules.end())
  {
    return false;
  }
  info = *it;
  return true;
}

bool DynamicInfo::find_library(uint16_t id, LibraryInfo& info)
{
  auto it = std::find_if(libraries.begin(), libraries.end(), [&id](LibraryInfo& l) { return l.id == id; });
  if (it == libraries.end())
  {
    return false;
  }
  info = *it;
  return true;
}

bool prepare_dynamic_info(elf::Elf64_Dyn* entry, size_t entry_count, DynamicInfo& info)
{
  if (entry == nullptr || !entry_count)
  {
    return false;
  }

  bool has_fingerprint = false,
    has_output_path = false,
    has_export_module = false,
    has_hash_table_offset = false,
    has_hash_table_size = false,
    has_pltgot = false,
    has_pltrel = false,
    has_pltrela_table_offset = false,
    has_pltrela_table_size = false,
    has_rela_table_offset = false,
    has_rela_table_size = false,
    has_rela_sizzze = false,
    has_string_table_offset = false,
    has_string_table_size = false,
    has_symbol_table_offset = false,
    has_symbol_table_size = false,
    has_symbol_size = false;

  auto end = &entry[entry_count];
  for (; entry < end; ++entry)
  {
    if (entry->d_tag == elf::DT_NULL)
    {
      break;
    }

    switch (entry->d_tag)
    {
      case elf::DT_NEEDED:
      case elf::DT_INIT:
      case elf::DT_FINI:
      case elf::DT_SONAME:
      case elf::DT_SYMBOLIC:
      case elf::DT_DEBUG:
      case elf::DT_TEXTREL:
      case elf::DT_INIT_ARRAY:
      case elf::DT_FINI_ARRAY:
      case elf::DT_INIT_ARRAYSZ:
      case elf::DT_FINI_ARRAYSZ:
      case elf::DT_FLAGS:
      case elf::DT_PREINIT_ARRAY:
      case elf::DT_PREINIT_ARRAYSZ:
      case 0x60000005ll:
      case 0x6100000Fll:
      case 0x61000011ll:
      case 0x61000013ll:
      case 0x61000015ll:
      case 0x61000017ll:
      case 0x61000019ll:
      case elf::DT_RELACOUNT:
      case elf::DT_FLAGS_1:
      {
        break;
      }

      case elf::DT_PLTRELSZ:
      case 0x6100002Dll:
      {
        info.pltrela_table_size = entry->d_un.d_val;
        has_pltrela_table_size = true;
        break;
      }

      case elf::DT_RELASZ:
      case 0x61000031ll:
      {
        info.rela_table_size = entry->d_un.d_val;
        has_rela_table_size = true;
        break;
      }

      case elf::DT_RELAENT:
      case 0x61000033ll:
      {
        if (sizeof(elf::Elf64_Rela) != entry->d_un.d_val)
        {
          return false;
        }
        has_rela_sizzze = true;
        break;
      }

      case elf::DT_STRSZ:
      case 0x61000037ll:
      {
        info.string_table_size = entry->d_un.d_val;
        has_string_table_size = true;
        break;
      }

      case elf::DT_SYMENT:
      case 0x6100003Bll:
      {
        if (sizeof(elf::Elf64_Sym) != entry->d_un.d_val)
        {
          return false;
        }
        has_symbol_size = true;
        break;
      }

      case 0x61000025ll:
      {
        info.hash_table_offset = entry->d_un.d_val;
        has_hash_table_offset = true;
        break;
      }

      case 0x61000029ll:
      {
        info.pltrela_table_offset = entry->d_un.d_val;
        has_pltrela_table_offset = true;
        break;
      }

      case elf::DT_PLTREL:
      case 0x6100002Bll:
      {
        if (entry->d_un.d_val != elf::DT_RELA)
        {
          return false;
        }
        has_pltrel = true;
        break;
      }

      case 0x6100002Fll:
      {
        info.rela_table_offset = entry->d_un.d_val;
        has_rela_table_offset = true;
        break;
      }

      case 0x61000035ll:
      {
        info.string_table_offset = entry->d_un.d_val;
        has_string_table_offset = true;
        break;
      }

      case 0x61000039ll:
      {
        info.symbol_table_offset = entry->d_un.d_val;
        has_symbol_table_offset = true;
        break;
      }

      case 0x6100003Dll:
      {
        info.hash_table_size = entry->d_un.d_val;
        has_hash_table_size = true;
        break;
      }

      case 0x6100003Fll:
      {
        info.symbol_table_size = entry->d_un.d_val;
        has_symbol_table_size = true;
        break;
      }

      case 0x61000007ll:
      {
        has_fingerprint = true;
        break;
      }

      case 0x61000009ll:
      {
        has_output_path = true;
        break;
      }

      case 0x6100000Dll:
      {
        has_export_module = true;
        break;
      }

      case elf::DT_PLTGOT:
      case 0x61000027:
      {
        has_pltgot = true;
        break;
      }

      default:
      {
        return false;
      }
    }
  }

  return has_fingerprint && has_output_path && has_export_module &&
    has_pltgot && has_pltrel &&
    has_pltrela_table_offset && has_pltrela_table_size &&
    has_rela_table_offset && has_rela_table_size && has_rela_sizzze &&
    has_string_table_offset && has_string_table_size &&
    has_symbol_table_offset && has_symbol_table_size && has_symbol_size &&
    has_hash_table_offset && has_hash_table_size;
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

bool uplift::get_dynamic_info(elf::Elf64_Dyn* entry, size_t entry_count, uint8_t* data_buffer, size_t data_size, DynamicInfo& info)
{
  if (!prepare_dynamic_info(entry, entry_count, info))
  {
    return false;
  }

  StringTable string_table =
  {
    reinterpret_cast<const char*>(&data_buffer[info.string_table_offset]),
    info.string_table_size,
  };

  if (entry == nullptr || !entry_count)
  {
    return false;
  }

  auto end = &entry[entry_count];
  for (; entry < end; ++entry)
  {
    if (entry->d_tag == elf::DT_NULL)
    {
      break;
    }

    switch (entry->d_tag)
    {
      case elf::DT_NEEDED:
      {
        auto s = string_table.get(entry->d_un.d_val);
        if (!s)
        {
          return false;
        }
        info.shared_object_names.push_back(xe::to_wstring(s));
        break;
      }

      case elf::DT_PLTGOT:
      case 0x61000027ll:
      {
        info.pltgot_offset = entry->d_un.d_val;
        break;
      }

      case elf::DT_INIT:
      {
        info.init_offset = entry->d_un.d_val;
        info.has_init_offset = true;
        break;
      }

      case elf::DT_FINI:
      {
        info.fini_offset = entry->d_un.d_val;
        info.has_fini_offset = true;
        break;
      }

      case elf::DT_SONAME:
      {
        auto s = string_table.get(entry->d_un.d_val);
        info.shared_object_name = !s ? L"" : xe::to_wstring(s);
        break;
      }

      case elf::DT_SYMBOLIC:
      {
        info.flags |= DynamicFlags::IsSymbolic;
        break;
      }

      case elf::DT_TEXTREL:
      {
        info.flags |= DynamicFlags::HasTextRelocations;
        break;
      }

      case elf::DT_FLAGS:
      {
        auto flags = entry->d_un.d_val;
        if (flags & elf::DF_SYMBOLIC)
        {
          info.flags |= DynamicFlags::IsSymbolic;
        }
        if (flags & elf::DF_TEXTREL)
        {
          info.flags |= DynamicFlags::HasTextRelocations;
        }
        if (flags & elf::DF_BIND_NOW)
        {
          info.flags |= DynamicFlags::BindNow;
        }
        break;
      }

      case 0x61000007ll:
      {
        if (entry->d_un.d_val + 20 <= data_size)
        {
          std::memcpy(info.fingerprint, &data_buffer[entry->d_un.d_val], 20);
        }
        break;
      }

      case 0x61000009ll:
      {
        auto s = string_table.get(entry->d_un.d_val);
        if (!s)
        {
          return false;
        }
        info.output_image_name = std::string(s);
        break;
      }

      case 0x6100000Dll:
      case 0x6100000Fll:
      {
        ModuleInfo module;
        module.value = entry->d_un.d_val;
        module.attributes = 0;
        module.name = string_table.get(module.name_offset);
        info.modules.push_back(module);
        break;
      }

      case 0x61000011ll:
      {
        union
        {
          uint64_t value;
          struct
          {
            uint16_t attributes;
            uint16_t unknown_02;
            uint16_t unknown_04;
            uint16_t id;
          };
        } flags = { entry->d_un.d_val };

        auto it = std::find_if(info.modules.begin(), info.modules.end(), [&flags](ModuleInfo& m) { return m.id == flags.id; });
        if (it == info.modules.end())
        {
          return false;
        }
        auto index = std::distance(info.modules.begin(), it);
        auto module = info.modules[index];
        module.attributes = flags.attributes;
        info.modules[index] = module;
        break;
      }

      case 0x61000013ll:
      case 0x61000015ll:
      {
        LibraryInfo library;
        library.value = entry->d_un.d_val;
        library.attributes = 0;
        library.is_export = entry->d_tag == 0x61000013ll;
        library.name = string_table.get(library.name_offset);
        info.libraries.push_back(library);
        break;
      }

      case 0x61000017ll:
      case 0x61000019ll:
      {
        union
        {
          uint64_t value;
          struct
          {
            uint16_t attributes;
            uint16_t unknown_02;
            uint16_t unknown_04;
            uint16_t id;
          };
        } flags = { entry->d_un.d_val };

        auto it = std::find_if(info.libraries.begin(), info.libraries.end(), [&flags](LibraryInfo& l) { return l.id == flags.id; });
        if (it == info.libraries.end())
        {
          return false;
        }
        auto index = std::distance(info.libraries.begin(), it);
        auto library = info.libraries[index];
        library.attributes = flags.attributes;
        info.libraries[index] = library;
        break;
      }

      case elf::DT_FLAGS_1:
      {
        auto flags = entry->d_un.d_val;
        if (flags & elf::DF_1_NOW)
        {
          info.flags |= DynamicFlags::BindNow;
        }
        if (flags & elf::DF_1_NODELETE)
        {
          info.flags |= DynamicFlags::NoDelete;
        }
        if (flags & elf::DF_1_LOADFLTR)
        {
          info.flags |= DynamicFlags::LoadFilter;
        }
        if (flags & elf::DF_1_NOOPEN)
        {
          info.flags |= DynamicFlags::NoOpen;
        }
        break;
      }

      case elf::DT_PLTRELSZ:
      case elf::DT_RELASZ:
      case elf::DT_RELAENT:
      case elf::DT_STRSZ:
      case elf::DT_SYMENT:
      case elf::DT_PLTREL:
      case elf::DT_DEBUG:
      case elf::DT_INIT_ARRAY:
      case elf::DT_FINI_ARRAY:
      case elf::DT_INIT_ARRAYSZ:
      case elf::DT_FINI_ARRAYSZ:
      case elf::DT_PREINIT_ARRAY:
      case elf::DT_PREINIT_ARRAYSZ:
      case 0x60000005ll:
      case 0x61000025ll:
      case 0x61000029ll:
      case 0x6100002Bll:
      case 0x6100002Dll:
      case 0x6100002Fll:
      case 0x61000031ll:
      case 0x61000033ll:
      case 0x61000035ll:
      case 0x61000037ll:
      case 0x61000039ll:
      case 0x6100003Bll:
      case 0x6100003Dll:
      case 0x6100003Fll:
      case elf::DT_RELACOUNT:
      {
        break;
      }

      default:
      {
        return false;
      }
    }
  }

  return true;
}
