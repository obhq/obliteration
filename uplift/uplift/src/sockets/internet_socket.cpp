#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "internet_socket.hpp"

#ifdef XE_PLATFORM_WIN32
#include "xenia/base/platform_win.h"
#include <WS2tcpip.h>
#include <WinSock2.h>
#else
#error todo
#endif

using namespace uplift;
using namespace uplift::sockets;
using namespace uplift::syscall_errors;

using Domain = Socket::Domain;
using Type = Socket::Type;
using Protocol = Socket::Protocol;

struct native_dtp
{
  int af;
  int type;
  int protocol;
};

bool translate_dtp(Type type, Protocol protocol, native_dtp& native_dtp)
{
  switch (type)
  {
    case Type::Stream:
    {
      switch (protocol)
      {
        case Protocol::Default:
        {
          native_dtp = { AF_INET, SOCK_STREAM, IPPROTO_TCP };
          return true;
        }
      }
    }
    case Type::Datagram:
    case Type::DatagramP2P:
    {
      switch (protocol)
      {
        case Protocol::Default:
        {
          native_dtp = { AF_INET, SOCK_DGRAM, IPPROTO_UDP };
          return true;
        }
      }
    }
  }
  assert_always();
  return false;
}

InternetSocket::InternetSocket(Runtime* runtime)
  : Socket(runtime)
  , native_handle_(-1)
  , type_(Type::Invalid)
  , protocol_(Protocol::Invalid)
{
}

InternetSocket::InternetSocket(Runtime* runtime, uint32_t native_handle)
  : Socket(runtime)
  , native_handle_(native_handle) 
{
}

InternetSocket::~InternetSocket()
{
  Close(); 
}

SCERR InternetSocket::Initialize(Domain domain, Type type, Protocol protocol)
{
  if (domain != Domain::IPv4)
  {
    return SCERR::eINVAL;
  }

  native_dtp native_dtp;
  if (!translate_dtp(type, protocol, native_dtp))
  {
    return SCERR::eINVAL;
  }

  auto native_handle = socket(native_dtp.af, native_dtp.type, native_dtp.protocol);
  if (native_handle == INVALID_SOCKET)
  {
    auto err = WSAGetLastError();
    return SCERR::eNOMEM;
  }

  type_ = type;
  protocol_ = protocol;
  native_handle_ = native_handle;
  return SUCCESS;
}

SCERR InternetSocket::Close()
{
#if XE_PLATFORM_WIN32
  int result = closesocket(native_handle_);
#elif XE_PLATFORM_LINUX
  int result = close(native_handle_);
#endif
  return result != 0 ? SCERR::eIO : SUCCESS;
}

SCERR InternetSocket::Connect(const void* name, uint32_t namelen)
{
  return SCERR::eINVAL;
}

SCERR InternetSocket::Read(void* data_buffer, size_t data_size, size_t* read_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR InternetSocket::Write(const void* data_buffer, size_t data_size, size_t* written_size)
{
  assert_always();
  return SCERR::eNODEV;
}

SCERR InternetSocket::IOControl(uint32_t request, void* argp)
{
  switch (request)
  {
    case 0x802450C9u: // init socket subsystem?
    {
      return SUCCESS;
    }
  }

  assert_always();
  return SCERR::eNODEV;
}

SCERR InternetSocket::MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation)
{
  assert_always();
  return SCERR::eNODEV;
}
