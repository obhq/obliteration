#include "stdafx.h"

#include <xenia/base/exception_handler.h>
#include <xenia/base/socket.h>
#include <xenia/base/string.h>

#include <xbyak/xbyak_util.h>

#include <WinSock2.h>

#include "runtime.hpp"
#include "module.hpp"

int main(int argc, char* argv[])
{
#ifdef XE_PLATFORM_WIN32
  WSADATA wsa_data;
  WSAStartup(MAKEWORD(2, 2), &wsa_data);
#endif

  uplift::Runtime runtime;

  bool missing_feature = false;
#define CHECK_FEATURE(x,y) \
  if (!runtime.cpu_has(Xbyak::util::Cpu::t ## x)) \
  { \
    printf("Your CPU does not support " y ".\n"); \
    missing_feature = true; \
  }
  /* Check necessary CPU features.
   * https://en.wikipedia.org/wiki/Jaguar_(microarchitecture)#Instruction_set_support
   * Not all Jaguar features are actually available, just a subset.
   * Features checks left commented out can be simulated by the runtime, when necessary.
   */
  CHECK_FEATURE(SSE, "SSE");
  CHECK_FEATURE(SSE2, "SSE2");
  CHECK_FEATURE(SSE3, "SSE3");
  CHECK_FEATURE(SSSE3, "SSSE3");
  CHECK_FEATURE(SSE41, "SSE4.1");
  CHECK_FEATURE(SSE42, "SSE4.2");
  CHECK_FEATURE(AESNI, "AES");
  CHECK_FEATURE(AVX, "AVX");
  //CHECK_FEATURE(SSE4a, "SSE4a");
  //CHECK_FEATURE(BMI1, "BMI1");
  CHECK_FEATURE(PCLMULQDQ, "CLMUL");
  CHECK_FEATURE(F16C, "F16C");
  //CHECK_FEATURE(MOVBE, "MOVBE");
#undef CHECK_FEATURE
  if (missing_feature)
  {
    return 1;
  }

  if (argc < 2)
  {
    return 2;
  }

  auto boot_path = xe::to_absolute_path(xe::to_wstring(argv[1]));

  auto base_path = xe::find_base_path(boot_path);
  runtime.set_base_path(base_path);

  if (!runtime.LoadExecutable(xe::find_name_from_path(boot_path)))
  {
    return 3;
  }

  std::vector<std::string> args;
  args.push_back("");
  args.push_back("");
  args.push_back("");
  args.push_back("");

  auto handle_exception = [](xe::Exception* ex, void* data)
  {
    return static_cast<uplift::Runtime*>(data)->HandleException(ex);
  };
  xe::ExceptionHandler::Install(handle_exception, &runtime);
  runtime.Run(args);
  xe::ExceptionHandler::Uninstall(handle_exception, &runtime);
  return 0;
}
