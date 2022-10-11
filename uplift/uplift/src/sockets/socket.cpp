#include "stdafx.h"

#include "../runtime.hpp"
#include "socket.hpp"

using namespace uplift;
using namespace uplift::sockets;

Socket::Socket(Runtime* runtime)
  : Object(runtime, ObjectType)
{
}

Socket::~Socket()
{
}
