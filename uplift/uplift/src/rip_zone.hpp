#pragma once

#include <cstdint>

namespace uplift
{
  struct RIPZone
  {
    uint8_t* base_address;
    uint8_t* current_address;
    uint8_t* end_address;

    bool take(size_t size, uint8_t*& result)
    {
      auto next_address = &current_address[size];
      if (next_address < base_address || next_address > end_address)
      {
        return false;
      }
      result = current_address;
      current_address = next_address;
      return true;
    }
  };
}
