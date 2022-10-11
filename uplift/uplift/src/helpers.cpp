#include "stdafx.h"

#define NOMINMAX
#include <windows.h>

#include <llvm/BinaryFormat/ELF.h>

#include <xenia/base/assert.h>
#include <xenia/base/memory.h>
#include <xenia/base/mapped_memory.h>

#include "helpers.hpp"

#include "match.hpp"

using namespace uplift;
namespace elf = llvm::ELF;

xe::memory::PageAccess uplift::get_page_access(elf::Elf64_Word flags)
{
  switch (flags & (elf::PF_R | elf::PF_W | elf::PF_X))
  {
    case elf::PF_R:
    {
      return xe::memory::PageAccess::kReadOnly;
    }

    case elf::PF_R | elf::PF_W:
    {
      return xe::memory::PageAccess::kReadWrite;
    }

    case elf::PF_X:
    {
      return xe::memory::PageAccess::kExecuteOnly;
    }

    case elf::PF_X | elf::PF_R:
    {
      return xe::memory::PageAccess::kExecuteRead;
    }

    case elf::PF_X | elf::PF_R | elf::PF_W:
    {
      return xe::memory::PageAccess::kExecuteReadWrite;
    }
  }
  assert_unhandled_case(flags);
  return xe::memory::PageAccess::kNoAccess;
}

bool get_executable_text_region(uint8_t* buffer, size_t buffer_size, uint8_t*& text, size_t& text_size)
{
  const char interp[] = "/libexec/ld-elf.so.1";

  auto start = buffer;
  auto current = buffer;
  auto end = &buffer[buffer_size];

#define SKIP_NULLS \
  for (; current < end && *current == 0; ++current) {}

  if (&current[sizeof(interp)] <= end)
  {
    if (memcmp(buffer, interp, sizeof(interp)) == 0)
    {
      current += sizeof(interp);
      SKIP_NULLS;
      start = current;
    }
  }

  unsigned short init_pattern[] =
  {
    0x48, 0x85, 0xC0,
    0x74, 0xF4,
    0x48, 0x83, 0xF8, 0xFF,
    0x74, 0x04,
    0xFF, 0xD0,
    0xEB, 0xEA,
    0x48, 0x83, 0xC4, 0x08,
    0x5B,
    0x41, 0x5E,
    0x41, 0x5F,
    0x5D,
    0xC3,
  };
  void* init_end = nullptr;
  if (!match_buffer(current, static_cast<size_t>(end - current), init_pattern, _countof(init_pattern), &init_end))
  {
    return false;
  }
  current = static_cast<uint8_t*>(init_end) + _countof(init_pattern);

  SKIP_NULLS;

  unsigned short fini_pattern[] =
  {
    0x55,
    0x48, 0x89, 0xE5,
    0x53,
    0x50,
    0x8A, 0x05,
    MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    MATCH_ANY, MATCH_ANY,
    0x75, 0x35,
    0x48, 0x8B, 0x05,
    MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    0x48, 0x85, 0xC0,
    0x74, 0x22,
    0x48, 0x8D, 0x1D,
    MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    0x66, 0x66, 0x66, 0x66, 0x2E, 0x0F, 0x1F, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xFF, 0xD0,
    0x48, 0x8B, 0x03,
    0x48, 0x83, 0xC3, 0x08,
    0x48, 0x85, 0xC0,
    0x75, 0xF2,
    0xC6, 0x05,
    MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    0x01,
    0x48, 0x83, 0xC4, 0x08,
    0x5B,
    0x5D,
    0xC3,
  };
  void* fini_end = nullptr;
  if (!match_buffer(current, static_cast<size_t>(end - current), fini_pattern, _countof(fini_pattern), &fini_end))
  {
    return false;
  }
  current = static_cast<uint8_t*>(fini_end) + _countof(fini_pattern);

  SKIP_NULLS;

  if (current + 16 > end)
  {
    return false;
  }

  if (current[0] != 0xFF ||
    current[1] != 0x35 ||
    current[6] != 0xFF ||
    current[7] != 0x25 ||
    current[12] != 0x90 ||
    current[13] != 0x90 ||
    current[14] != 0x90 ||
    current[15] != 0x90)
  {
    return false;
  }
  current += 16;

  for (; current + 16 <= end; current += 16)
  {
    if (current[0] != 0xFF ||
      current[1] != 0x25 ||
      current[6] != 0x68 ||
      current[11] != 0xE9)
    {
      break;
    }
  }

#undef SKIP_NULLS

  text = start;
  text_size = static_cast<size_t>(current - start);

  return true;
}

bool get_shared_object_text_region(uint8_t* buffer, size_t buffer_size, uint8_t*& text, size_t& text_size)
{
  auto start = buffer;
  auto current = buffer;
  auto end = &buffer[buffer_size];

#define SKIP_NULLS \
  for (; current < end && *current == 0; ++current) {}

  unsigned short init_pattern[] =
  {
    0x31, 0xC0,
    0x48, 0x83, 0xC4, 0x08,
    0x5B,
    0x41, 0x5C,
    0x41, 0x5D,
    0x41, 0x5E,
    0x41, 0x5F,
    0x5D,
    0xC3,
  };
  void* init_end = nullptr;
  if (!match_buffer(current, static_cast<size_t>(end - current), init_pattern, _countof(init_pattern), &init_end))
  {
    return false;
  }
  current = static_cast<uint8_t*>(init_end) + _countof(init_pattern);

  SKIP_NULLS;

  if (current + 16 > end)
  {
    return false;
  }

  bool is_aligned = false;
check_again:
  if (current[0] != 0xFF ||
    current[1] != 0x35 ||
    current[6] != 0xFF ||
    current[7] != 0x25 ||
    current[12] != 0x90 ||
    current[13] != 0x90 ||
    current[14] != 0x90 ||
    current[15] != 0x90)
  {
    if (is_aligned == false)
    {
      is_aligned = true;
      current = reinterpret_cast<uint8_t*>((reinterpret_cast<uint64_t>(current) + 0xF) & ~0xF);
      goto check_again;
    }

    return false;
  }
  current += 16;

  for (; current + 16 <= end; current += 16)
  {
    if (current[0] != 0xFF ||
      current[1] != 0x25 ||
      current[6] != 0x68 ||
      current[11] != 0xE9)
    {
      break;
    }
  }

  unsigned short fini_pattern[] =
  {
    0x55,
    0x48, 0x89, 0xE5,
    0x41, 0x56,
    0x53,
    MATCH_ANY, MATCH_ANY, MATCH_ANY,
    MATCH_ANY, MATCH_ANY,
    MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    MATCH_ANY, MATCH_ANY,
    0x75, 0x61,
    0x48, 0x85, 0xD2,
    0x74, 0x04,
    0xFF, 0xD2,
    0xEB, 0x12,
    0x45, 0x31, 0xF6,
    0x48, 0x83, 0x3D, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY, 0x00,
    0x74, 0x08,
    0xE8, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    0x41, 0x89, 0xC6,
    0x48, 0x83, 0x3D, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY, 0x00,
    0x74, 0x0F,
    0x48, 0x8D, 0x05, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    0x48, 0x8B, 0x38,
    0xE8, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    0x48, 0x8B, 0x05, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    0x48, 0x85, 0xC0,
    0x74, 0x17,
    0x48, 0x8D, 0x1D, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY,
    0x66, 0x90,
    0xFF, 0xD0,
    0x48, 0x8B, 0x03,
    0x48, 0x83, 0xC3, 0x08,
    0x48, 0x85, 0xC0,
    0x75, 0xF2,
    0xC6, 0x05, MATCH_ANY, MATCH_ANY, MATCH_ANY, MATCH_ANY, 0x01,
    0x44, 0x89, 0xF0,
    0x5B,
    0x41, 0x5E,
    0x5D,
    0xC3,
  };
  void* fini_end = nullptr;
  if (!match_buffer(current, static_cast<size_t>(end - current), fini_pattern, _countof(fini_pattern), &fini_end))
  {
    return false;
  }
  current = static_cast<uint8_t*>(fini_end) + _countof(fini_pattern);

  SKIP_NULLS;

#undef SKIP_NULLS

  text = start;
  text_size = static_cast<size_t>(current - start);

  return true;
}

bool uplift::get_text_region(uint8_t* buffer, size_t buffer_size, uint8_t*& text, size_t& text_size)
{
  return get_executable_text_region(buffer, buffer_size, text, text_size) ||
    get_shared_object_text_region(buffer, buffer_size, text, text_size);
}

bool decode_value(std::string buffer, uint64_t& value)
{
  const char codes[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";
  value = 0;
  for (int i = 0; i < buffer.size(); ++i)
  {
    auto code = strchr(codes, buffer[i]);
    uint64_t index;
    if (code == nullptr)
    {
      return false;
    }
    else
    {
      index = static_cast<uint64_t>(code - codes);
    }
    value <<= 6;
    value |= index;
  }
  return true;
}

bool uplift::parse_symbol_name(const std::string& buffer, std::string& name, uint16_t& library_id, uint16_t& module_id)
{
  auto library_index = buffer.find('#');
  if (library_index != std::string::npos)
  {
    auto module_index = buffer.find('#', library_index + 1);
    if (module_index != std::string::npos)
    {
      if ((module_index - library_index) <= 4 &&
          (buffer.size() - module_index) <= 4)
      {
        uint64_t library_id_dummy, module_id_dummy;
        if (!decode_value(buffer.substr(library_index + 1, (module_index - (library_index + 1))), library_id_dummy) ||
            !decode_value(buffer.substr(module_index + 1), module_id_dummy))
        {
          return false;
        }
        name = buffer.substr(0, library_index);
        library_id = static_cast<uint16_t>(library_id_dummy);
        module_id = static_cast<uint16_t>(module_id_dummy);
        return true;
      }
    }
  }
  return false;
}

uint32_t uplift::elf_hash(const char* name)
{
  auto p = (const uint8_t*)name;
  uint32_t h = 0;
  uint32_t g;
  while (*p != '\0')
  {
    h = (h << 4) + *p++;
    if ((g = h & 0xF0000000ull) != 0)
    {
      h ^= g >> 24;
    }
    h &= ~g;
  }
  return h;
}
