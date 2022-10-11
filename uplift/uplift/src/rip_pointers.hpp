#pragma once

namespace uplift
{
#pragma pack(push, 16)
  struct RIPPointers
  {
    void* fsbase;
    void* runtime;
    void* syscall_handler;
  };
#pragma pack(pop)
}
