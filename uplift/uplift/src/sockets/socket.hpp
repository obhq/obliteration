#pragma once

#include "../objects/object.hpp"

namespace uplift::sockets
{
  class Socket : public objects::Object
  {
  public:
    static const Object::Type ObjectType = Type::Socket;

    enum class Domain : int32_t
    {
      Invalid = -1,
      Unix = 1,
      IPv4 = 2,
    };

    enum class Type : int32_t
    {
      Invalid = -1,
      Stream = 1,
      Datagram = 2,
      DatagramP2P = 6,
    };

    enum class Protocol : int32_t
    {
      Invalid = -1,
      Default = 0,
      TCP = 6,
      UDP = 17,
    };

  protected:
    Socket(Runtime* runtime);

  public:
    virtual ~Socket();

    virtual SyscallError Initialize(Domain domain, Type type, Protocol protocol) = 0;
    virtual SyscallError Connect(const void* name, uint32_t namelen) = 0;
  };
}
