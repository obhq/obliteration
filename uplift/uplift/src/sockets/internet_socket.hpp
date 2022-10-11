#pragma once

#include "socket.hpp"

namespace uplift::sockets
{
  class InternetSocket : public Socket
  {
  public:
    InternetSocket(Runtime* runtime);
    virtual ~InternetSocket();

    uint64_t native_handle() const { return native_handle_; }

    SyscallError Initialize(Socket::Domain domain, Socket::Type type, Socket::Protocol protocol);
    SyscallError Connect(const void* name, uint32_t namelen);

    SyscallError Close();
    SyscallError Read(void* data_buffer, size_t data_size, size_t* read_size);
    SyscallError Write(const void* data_buffer, size_t data_size, size_t* written_size);
    SyscallError IOControl(uint32_t request, void* argp);
    SyscallError MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation);

  private:
    InternetSocket(Runtime* runtime, uint32_t native_handle);
    uint64_t native_handle_ = -1;

    Type type_;
    Protocol protocol_;
  };
}
