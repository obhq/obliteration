#include "stdafx.h"

#include <xenia/base/memory.h>
#include <xenia/base/string.h>

#include "runtime.hpp"
#include "syscalls.hpp"
#include "syscall_errors.hpp"
#include "helpers.hpp"

#include "objects/_objects.hpp"
#include "devices/_devices.hpp"
#include "ipmi/_ipmi.h"
#include "sockets/_sockets.hpp"

using namespace uplift;
using namespace uplift::syscall_errors;
using namespace uplift::devices;
using namespace uplift::ipmi;
using namespace uplift::objects;
using namespace uplift::sockets;

#define SYSCALL_IMPL(x, ...) bool SYSCALLS::x(Runtime* runtime, SyscallReturnValue& retval, __VA_ARGS__)

SYSCALL_IMPL(exit, int status)
{
  // exit syscall probably needs special handling to jump to .fini/termination code directly
  assert_always();
  retval.val = -1;
  return false;
}

SYSCALL_IMPL(write, uint32_t fd, const void* buf, size_t nbytes)
{
  if (fd == 1 || fd == 2) // stdout, stderr
  {
    auto b = static_cast<const char*>(buf);
    for (size_t i = 0; i < nbytes; ++i, ++b)
    {
      printf("%c", *b);
    }
    retval.val = nbytes;
    return true;
  }
  else
  {
    auto object = runtime->object_table()->LookupObject(static_cast<ObjectHandle>(fd)).get();
    if (object)
    {
      size_t written;
      auto result = object->Write(buf, nbytes, &written);
      if (!IS_SUCCESS(result))
      {
        retval.err = result;
        return false;
      }
      retval.val = written;
      return true;
    }
  }

  retval.err = SCERR::eBADF;
  assert_always();
  return false;
}

SCERR open_device(Runtime* runtime, const char* path, uint32_t flags, uint32_t mode, ObjectHandle& handle)
{
  Device* device = nullptr;
  const char* name = &path[5];
  if (!strcmp(name, "console"))
  {
    device = object_ref<ConsoleDevice>(new ConsoleDevice(runtime)).get();
  }
  else if (!strcmp(name, "deci_tty6"))
  {
    device = object_ref<DeciTTYDevice>(new DeciTTYDevice(runtime)).get();
  }
  else if (!strncmp(name, "dmem", strlen("dmem")))
  {
    device = object_ref<DirectMemoryDevice>(new DirectMemoryDevice(runtime)).get();
  }
  else if (!strcmp(name, "dipsw"))
  {
    device = object_ref<DipswDevice>(new DipswDevice(runtime)).get();
  }
  else if (!strcmp(name, "gc"))
  {
    device = object_ref<GCDevice>(new GCDevice(runtime)).get();
  }
  else if (!strncmp(name, "notification", strlen("notification")))
  {
    device = object_ref<NotificationDevice>(new NotificationDevice(runtime)).get();
  }
  else
  {
    device = nullptr;
  }

  if (!device)
  {
    return SCERR::eNOENT;
  }

  auto result = device->Initialize(std::string(path), flags, mode);
  if (IS_ERROR(result))
  {
    device->ReleaseHandle();
    return result;
  }

  handle = device->handle();
  return SUCCESS;
}

SYSCALL_IMPL(open, const char* path, uint32_t flags, uint32_t mode)
{
  printf("open: %s, %x, %x\n", path, flags, mode);

  if (path == nullptr)
  {
    retval.err = SCERR::eINVAL;
    return false;
  }

  if (!strncmp(path, "/dev/", 5))
  {
    ObjectHandle handle;
    auto result = open_device(runtime, path, flags, mode, handle);
    if (IS_ERROR(result))
    {
      retval.err = result;
      return false;
    }
    retval.val = handle;
    return true;
  }

  if (!strcmp(path, "/app0/sce_discmap.plt") ||
      !strcmp(path, "/app0/sce_discmap_patch.plt"))
  {
    // short circuit some files not cared about yet
    retval.err = SCERR::eBUSY;
    return false;
  }

  assert_always();
  retval.err = SCERR::eBUSY;
  return false;
}

SYSCALL_IMPL(close, uint32_t fd)
{
  auto object = runtime->object_table()->LookupObject(static_cast<ObjectHandle>(fd)).get();
  if (!object)
  {
    assert_always();
    retval.err = SCERR::eBADF;
    return false;
  }
  object->Close();
  object->ReleaseHandle();
  return true;
}

SYSCALL_IMPL(getpid)
{
  retval.val = 123;
  return true;
}

SYSCALL_IMPL(ioctl, uint32_t fd, uint32_t request, void* argp)
{
  const char* labels[] = { "!", "void", "out", "void+out", "in", "void+in", "out+in", "void+out+in" };
  auto label = labels[(request >> 29) & 7u];
  printf("ioctl(%d): [%x] inout=%s, group=%c, num=%u, len=%u\n",
         fd, request, label, (request >> 8) & 0xFFu, request & 0xFFu, (request >> 16) & 0x1FFFu);

  auto object = runtime->object_table()->LookupObject(static_cast<ObjectHandle>(fd)).get();
  if (object)
  {
    retval.err = object->IOControl(request, argp);
    return IS_SUCCESS(retval.err);
  }

  assert_always();
  retval.err = SCERR::eBADF;
  return false;
}

SYSCALL_IMPL(munmap, void* addr, size_t len)
{
  printf("munmap: %p-%p (%I64u)\n", addr, &static_cast<const uint8_t*>(addr)[(!len ? 1 : len) - 1], len);
  return true;
}

SYSCALL_IMPL(mprotect, const void* addr, size_t len, int prot)
{
  printf("mprotect: %p-%p (%I64u) %x\n", addr, &static_cast<const uint8_t*>(addr)[(!len ? 1 : len) - 1], len, prot);
  return true;
}

SYSCALL_IMPL(socket, int domain, int type, int protocol)
{
  object_ref<Socket> socket;
  switch (static_cast<Socket::Domain>(domain))
  {
    case Socket::Domain::IPv4:
    {
      socket = object_ref<InternetSocket>(new InternetSocket(runtime));
      break;
    }

    default:
    {
      retval.err = SCERR::eINVAL;
      return false;
    }
  }

  auto result = socket->Initialize(
    static_cast<Socket::Domain>(domain),
    static_cast<Socket::Type>(type),
    static_cast<Socket::Protocol>(protocol));
  if (IS_ERROR(result))
  {
    socket->Release();
    retval.err = result;
    return false;
  }
  retval.val = socket->handle();
  return true;
}

SYSCALL_IMPL(connect, uint32_t s, const void* name, uint32_t namelen)
{
  if (namelen > 255)
  {
    retval.err = SCERR::eNAMETOOLONG;
    return false;
  }

  auto object = runtime->object_table()->LookupObject<Socket>(static_cast<ObjectHandle>(s));
  if (!object)
  {
    retval.err = SCERR::eBADF;
    return false;
  }

  retval.err = object->Connect(name, namelen);
  return IS_SUCCESS(retval.err);
}

SYSCALL_IMPL(netcontrol, uint32_t fd, uint32_t op, void* data_buffer, uint32_t data_size)
{
  if (data_size > 160)
  {
    retval.err = SCERR::eINVAL;
    return false;
  }

  assert_true(fd == -1);

  switch (op)
  {
    case 20: // bnet_get_secure_seed
    {
      *static_cast<uint32_t*>(data_buffer) = 4; // totally secure number
      return true;
    }
  }

  assert_always();
  retval.err = SCERR::eINVAL;
  return false;
}

SYSCALL_IMPL(socketex, const char* name, int domain, int type, int protocol)
{
  if (!SYSCALLS::socket(runtime, retval, domain, type, protocol))
  {
    return false;
  }
  runtime->object_table()->AddNameMapping(name, static_cast<ObjectHandle>(retval.val));
  return true;
}

SYSCALL_IMPL(socketclose, uint32_t s)
{
  return SYSCALLS::close(runtime, retval, s);
}

SYSCALL_IMPL(gettimeofday, void* tp, void* tzp)
{
  retval.val = -1;
  return false;
}

SYSCALL_IMPL(sysarch, int number, void* args)
{
  if (number == 129)
  {
    auto fsbase = *static_cast<void**>(args);
    printf("FSBASE=%p, %p\n", args, fsbase);
    runtime->set_fsbase(fsbase);
    return true;
  }
  assert_always();
  retval.val = -1;
  return false;
}

SYSCALL_IMPL(sysctl, int* name, uint32_t namelen, void* oldp, size_t* oldlenp, const void* newp, size_t newlen)
{
  if (namelen == 2 && name[0] == 0 && name[1] == 3)
  {
    auto name = std::string(static_cast<const char*>(newp), newlen);

    if (name == "kern.smp.cpus")
    {
      static_cast<uint32_t*>(oldp)[0] = 0x0BADF00D;
      static_cast<uint32_t*>(oldp)[1] = 1;
      *oldlenp = 8;
      return true;
    }
    else if (name == "kern.proc.ptc")
    {
      static_cast<uint32_t*>(oldp)[0] = 0x0BADF00D;
      static_cast<uint32_t*>(oldp)[1] = 2;
      *oldlenp = 8;
      return true;
    }
    else if (name == "machdep.tsc_freq")
    {
      static_cast<uint32_t*>(oldp)[0] = 0x0BADF00D;
      static_cast<uint32_t*>(oldp)[1] = 3;
      *oldlenp = 8;
      return true;
    }
    else if (name == "kern.sched.cpusetsize")
    {
      static_cast<uint32_t*>(oldp)[0] = 0x0BADF00D;
      static_cast<uint32_t*>(oldp)[1] = 4;
      *oldlenp = 8;
      return true;
    }
    else if (name == "vm.ps4dev.vm1.cpu.pt_total" ||
             name == "vm.ps4dev.vm1.cpu.pt_available" ||
             name == "vm.ps4dev.vm1.gpu.pt_total" ||
             name == "vm.ps4dev.vm1.gpu.pt_available" ||
             name == "vm.ps4dev.trcmem_total" ||
             name == "vm.ps4dev.trcmem_avail")
    {
      // devkit, testkit?
      // claim they don't exist
      retval.err = SCERR::eNOENT;
      return false;
    }

    assert_always();
    return false;
  }
  else if (namelen == 2 && name[0] == 1 && name[1] == 37)
  {
    auto length = *oldlenp;
    if (length > 256) length = 256;
    memset(oldp, 4, length);
    *oldlenp = length;
    return true;
  }
  else if (namelen == 2 && name[0] == 1 && name[1] == 33)
  {
    assert_true(*oldlenp == 8);
    *static_cast<void**>(oldp) = runtime->user_stack_end_;
    return true;
  }
  else if (namelen == 2 && name[0] == 0x0BADF00D && name[1] == 1)
  {
    assert_true(*oldlenp == 4);
    *reinterpret_cast<uint32_t*>(oldp) = 1;
    return true;
  }
  else if (namelen == 2 && name[0] == 0x0BADF00D && name[1] == 2)
  {
    assert_true(*oldlenp == 8);
    *reinterpret_cast<uint64_t*>(oldp) = 1357;
    return true;
  }
  else if (namelen == 2 && name[0] == 0x0BADF00D && name[1] == 3)
  {
    assert_true(*oldlenp == 8);
    *reinterpret_cast<uint64_t*>(oldp) = 16000000000;
    return true;
  }
  else if (namelen == 2 && name[0] == 0x0BADF00D && name[1] == 4)
  {
    assert_true(*oldlenp == 4);
    *reinterpret_cast<uint32_t*>(oldp) = 8;
    return true;
  }
  else if (namelen == 2 && name[0] == 6 && name[1] == 7)
  {
    assert_true(*oldlenp == 4);
    *reinterpret_cast<uint32_t*>(oldp) = 4096;
    return true;
  }
  else if (namelen == 4 && name[0] == 1 && name[1] == 14 && name[2] == 35)
  {
    assert_true(*oldlenp == 72);
    std::memset(oldp, 0, 72);
    return true;
  }
  else if (namelen == 3 && name[0] == 1 && name[1] == 14 && name[2] == 41)
  {
    assert_true(*oldlenp == 4);
    *static_cast<uint32_t*>(oldp) = 0;
    return true;
  }
  else if (namelen == 4 && name[0] == 1 && name[1] == 14 && name[2] == 44)
  {
    assert_true(*oldlenp == 16);
    std::memset(oldp, 0, 16);
    return true;
  }

  assert_always();
  return false;
}

SCERR clock_gettime_win(uint32_t clock_id, void* tp);
SYSCALL_IMPL(clock_gettime, uint32_t clock_id, void* tp)
{
  retval.err = clock_gettime_win(clock_id, tp);
  return IS_SUCCESS(retval.err);
}

SYSCALL_IMPL(sigprocmask)
{
  return true;
}

SYSCALL_IMPL(kqueue)
{
  auto queue = object_ref<Queue>(new Queue(runtime)).get();
  SCERR result = SUCCESS; // Init?
  if (IS_ERROR(result))
  {
    queue->ReleaseHandle();
    retval.err = SCERR::eAGAIN;
    return false;
  }
  retval.val = queue->handle();
  return true;
}

SYSCALL_IMPL(sigaction)
{
  return true;
}

SYSCALL_IMPL(thr_self, void** arg1)
{
  *arg1 = (void*)357;
  retval.val = 135;
  return true;
}

SYSCALL_IMPL(_umtx_op, void* obj, int op, uint32_t val, void* uaddr1, void* uaddr2)
{
  return true;
}

SYSCALL_IMPL(thr_set_name, uint32_t id, const char* name)
{
  printf("thr_set_name: %d=%s\n", id, name);
  return true;
}

SYSCALL_IMPL(rtprio_thread, int function, uint64_t lwpid, void* rtp)
{
  return true;
}

SYSCALL_IMPL(mmap, void* addr, size_t len, uint32_t prot, uint32_t flags, uint32_t fd, size_t offset)
{
  printf("mmap: addr=%p, len=%I64x, prot=%x, flags=%x, fd=%d, offset=%I64x", addr, len, prot, flags, fd, offset);

  assert_true(!(flags & ~(0x1 | 0x2 | 0x10 | 0x100 | 0x400 | 0x1000 | 0x2000)));

  if (flags & 0x400)
  {
    flags |= 0x1000;
  }

  void* allocation = nullptr;
  SCERR result = SUCCESS;
  if (fd != -1)
  {
    auto object = runtime->object_table()->LookupObject(static_cast<ObjectHandle>(fd)).get();
    if (!object)
    {
      result = SCERR::eBADF;
    }
    else
    {
      result = object->MMap(addr, len, prot, flags, offset, allocation);
    }
  }
  else
  {
    auto access = xe::memory::PageAccess::kReadWrite;
    auto allocation_type = xe::memory::AllocationType::kReserveCommit;

    allocation = xe::memory::AllocFixed(addr, len, allocation_type, access);
    if (!allocation && !(flags & 0x10))
    {
      // not fixed, try allocating again
      allocation = xe::memory::AllocFixed(nullptr, len, allocation_type, access);
    }

    if (!allocation)
    {
      result = SCERR::eNOMEM;
    }
  }

  if (IS_ERROR(result))
  {
    printf(", FAILURE\n");
    retval.err = result;
    return false;
  }

  printf(", RETVAL=%p\n", allocation);

  if (flags & 0x1000) // anonymous
  {
    std::memset(allocation, 0, len);
  }

  if (flags & 0x400) // stack
  {
    allocation = &static_cast<uint8_t*>(allocation)[len];
  }

  retval.ptr = allocation;
  return true;
}

struct nonsys_int
{
  union
  {
    uint64_t encoded_id;
    struct
    {
      uint8_t data[4];
      uint8_t table;
      uint8_t index;
      uint16_t checksum;
    }
    encoded_id_parts;
  };
  uint32_t unknown;
  uint32_t value;
};

SYSCALL_IMPL(ftruncate, uint32_t fd, int64_t length)
{
  printf("ftruncate: %x %I64x\n", fd, length);

  if (length < 0)
  {
    retval.err = SCERR::eINVAL;
    return false;
  }

  auto object = runtime->object_table()->LookupObject(static_cast<ObjectHandle>(fd)).get();
  if (!object)
  {
    retval.err = SCERR::eBADF;
    return false;
  }

  retval.err = object->Truncate(length);
  return IS_SUCCESS(retval.err);
}

SYSCALL_IMPL(shm_open, const char* path, uint32_t flags, uint16_t mode)
{
  printf("shm_open: %s %x %x\n", path, flags, mode);

  if ((flags & 0x3) != 0 && (flags & 0x3) != 2) // O_RDONLY or O_RDWR
  {
    retval.err = SCERR::eINVAL;
    return false;
  }

  if (flags & ~(0x3 | 0xE00)) // O_CREAT or O_TRUNC or O_EXCL
  {
    retval.err = SCERR::eINVAL;
    return false;
  }

  ObjectHandle handle;
  if (!runtime->object_table()->GetObjectByName(std::string(path), &handle))
  {
    if (!(flags & 0x200))
    {
      retval.err = SCERR::eSRCH;
      return false;
    }

    auto shm = object_ref<SharedMemory>(new SharedMemory(runtime)).get();
    SCERR result = shm->Initialize(std::string(path), flags, mode);
    if (IS_ERROR(result))
    {
      shm->ReleaseHandle();
      retval.err = SCERR::eAGAIN;
      return false;
    }

    handle = shm->handle();
    runtime->object_table()->AddNameMapping(std::string(path), handle);
  }
  else
  {
    runtime->object_table()->RetainHandle(handle);
  }

  retval.val = handle;
  return true;
}

SYSCALL_IMPL(cpuset_getaffinity, int32_t level, int32_t which, int32_t id, size_t setsize, uint64_t* mask)
{
  return true;
}

SYSCALL_IMPL(regmgr_call, uint32_t op, uint32_t id, void* result, void* value, uint64_t type)
{
  if (op == 25) // non-system get int
  {
    auto int_value = static_cast<nonsys_int*>(value);

    if (int_value->encoded_id == 0x0CAE671ADF3AEB34ull ||
        int_value->encoded_id == 0x338660835BDE7CB1ull)
    {
      int_value->value = 0;
      retval.val = 0;
      return true;
    }

    retval.val = 0x800D0203;
    return false;
  }

  retval.val = -1;
  return false;
}

SYSCALL_IMPL(evf_create, const char* name, uint32_t flags, uint64_t arg3)
{
  printf("evf_create: %s %x %I64x\n", name, flags, arg3);

  if ((flags & ~0x133u) != 0x0 || (flags & 0x3) == 0x3)
  {
    retval.err = SCERR::eINVAL;
    return false;
  }

  auto evf = object_ref<EventFlag>(new EventFlag(runtime)).get();
  SCERR result = evf->Initialize(flags, arg3);
  if (IS_ERROR(result))
  {
    evf->ReleaseHandle();
    retval.err = SCERR::eAGAIN;
    return false;
  }
  runtime->object_table()->AddNameMapping(std::string(name), evf->handle());
  retval.val = evf->handle();
  return true;
}

SYSCALL_IMPL(evf_delete, uint32_t handle)
{
  printf("evf_delete: %x\n", handle);
  auto evf = runtime->object_table()->LookupObject<EventFlag>(static_cast<ObjectHandle>(handle)).get();
  if (!evf)
  {
    assert_always();
    retval.err = SCERR::eBADF;
    return false;
  }
  evf->Close();
  evf->ReleaseHandle();
  return true;
}

SYSCALL_IMPL(evf_open, const char* name)
{
  printf("evf_open: %s\n", name);
  ObjectHandle handle;
  if (!runtime->object_table()->GetObjectByName(std::string(name), &handle))
  {
    retval.err = SCERR::eSRCH;
    return false;
  }
  auto object = runtime->object_table()->LookupObject<EventFlag>(handle).get();
  if (!object)
  {
    retval.err = SCERR::eSRCH;
    return false;
  }
  object->RetainHandle();
  retval.val = object->handle();
  return true;
}

SYSCALL_IMPL(osem_create, const char *name, uint32_t flags, uint32_t arg3, uint32_t arg4)
{
  printf("osem_create: %s %x %x %x\n", name, flags, arg3, arg4);

  auto osem = object_ref<Semaphore>(new Semaphore(runtime)).get();
  SCERR result = osem->Initialize(flags, arg3, arg4);
  if (IS_ERROR(result))
  {
    osem->ReleaseHandle();
    retval.err = SCERR::eAGAIN;
    return false;
  }
  runtime->object_table()->AddNameMapping(std::string(name), osem->handle());
  retval.val = osem->handle();
  return true;
}

SYSCALL_IMPL(osem_delete, uint32_t handle)
{
  printf("osem_delete: %x\n", handle);
  auto osem = runtime->object_table()->LookupObject<Semaphore>(static_cast<ObjectHandle>(handle)).get();
  if (!osem)
  {
    assert_always();
    retval.err = SCERR::eBADF;
    return false;
  }
  osem->Close();
  osem->ReleaseHandle();
  return true;
}

SYSCALL_IMPL(osem_open, const char* name)
{
  printf("osem_open: %s\n", name);
  ObjectHandle handle;
  if (!runtime->object_table()->GetObjectByName(std::string(name), &handle))
  {
    retval.err = SCERR::eSRCH;
    return false;
  }
  auto object = runtime->object_table()->LookupObject<Semaphore>(handle).get();
  if (!object)
  {
    retval.err = SCERR::eSRCH;
    return false;
  }
  object->RetainHandle();
  retval.val = object->handle();
  return true;
}

SYSCALL_IMPL(namedobj_create, const char* name, void* arg2, uint32_t arg3)
{
  printf("namedobj_create: %s %p %x\n", name, arg2, arg3);
  retval.val = ++runtime->next_namedobj_id_;
  return true;
}

SYSCALL_IMPL(namedobj_delete)
{
  return true;
}

SYSCALL_IMPL(dmem_container, uint32_t arg1)
{
  if (arg1 == -1)
  {
    return true;
  }

  assert_always();
  retval.val = -1;
  return false;
}

SYSCALL_IMPL(get_authinfo, void* arg1, void* arg2)
{
  std::memset(arg2, 0, 136);
  return true;
}

SYSCALL_IMPL(mname, uint8_t* arg1, size_t arg2, const char* name, void* arg4)
{
  printf("mname: %p-%p=%s\n", arg1, &arg1[arg2] - 1, name);
  return true;
}

SYSCALL_IMPL(dynlib_dlsym, uint32_t handle, const char* cname, void** sym)
{
  auto module = runtime->object_table()->LookupObject<Module>(handle).get();
  if (!module)
  {
    retval.val = -1;
    return false;
  }

  auto module_name = xe::to_string(module->name());
  auto index = module_name.rfind('.');
  if (index != std::string::npos)
  {
    module_name = module_name.substr(0, index);
  }

  auto name = std::string(cname);

  if (name == "module_start")
  {
    printf("DLSYM FOR module_start OF %s!\n", module_name.c_str());
  }

  auto symbol_name = name + "#" + module_name + "#" + module_name;
  uint64_t symbol_value;
  if (module->ResolveSymbol(elf_hash(symbol_name.c_str()), symbol_name, symbol_value))
  {
    *sym = reinterpret_cast<void*>(symbol_value);
    return true;
  }

  std::string symbol_part;
  if (name == "sceSysmodulePreloadModuleForLibkernel")
  {
    symbol_part = "DOO+zuW1lrE";
  }
  else
  {
    retval.val = -1;
    return false;
  }

  symbol_name = symbol_part + "#" + module_name + "#" + module_name;
  if (module->ResolveSymbol(elf_hash(symbol_name.c_str()), symbol_name, symbol_value))
  {
    *sym = reinterpret_cast<void*>(symbol_value);
    return true;
  }

  return false;
}

SYSCALL_IMPL(dynlib_get_list, uint32_t* handles, size_t max_count, size_t* count)
{
  auto modules = runtime->object_table()->GetObjectsByType<Module>();
  std::sort(modules.begin(), modules.end(), [](object_ref<Module> a, object_ref<Module> b) { return a->order() < b->order(); });
  size_t i = 0;
  for (auto it = modules.begin(); i < max_count && it != modules.end(); ++it, ++i)
  {
    *(handles++) = (*it)->handle();
  }
  *count = i;
  return true;
}

struct dynlib_info
{
  size_t struct_size;
  char name[256];
  void* text_address;
  uint32_t text_size;
  uint32_t text_flags;
  void* data_address;
  int data_size;
  uint32_t data_flags;
  uint8_t unknown_128[32];
  uint32_t unknown_148;
  uint8_t fingerprint[20];
};

SYSCALL_IMPL(dynlib_get_info, uint32_t handle, void* vinfo)
{
  if (static_cast<dynlib_info*>(vinfo)->struct_size != sizeof(dynlib_info))
  {
    retval.err = SCERR::eINVAL;
    return false;
  }

  auto module = runtime->object_table()->LookupObject<Module>(handle).get();
  if (!module)
  {
    retval.err = SCERR::eSRCH;
    return false;
  }

  auto name = xe::to_string(module->name());
  auto index = name.rfind('.');
  if (index != std::string::npos)
  {
    name = name.substr(0, index);
  }

  auto base_address = module->base_address();
  auto program_info = module->program_info();
  auto dynamic_info = module->dynamic_info();

  dynlib_info info = {};
  std::strncpy(info.name, name.c_str(), sizeof(info.name));
  info.struct_size = sizeof(dynlib_info);
  info.text_address = module->text_address();
  info.text_size = static_cast<uint32_t>(module->text_size());
  info.text_flags = 1 | 4; // R+X
  info.data_address = module->data_address();
  info.data_size = static_cast<uint32_t>(module->data_size());
  info.data_flags = 1 | 2; // R+W
  info.unknown_148 = 2;
  std::memcpy(vinfo, &info, sizeof(info));
  return true;
}

SYSCALL_IMPL(dynlib_load_prx, const char* cpath, void* arg2, uint32_t* handle, void* arg4)
{
  printf("LOAD PRX: %s, %p, %p, %p\n", cpath, arg2, handle, arg4);

  auto path = xe::to_wstring(cpath);

  auto index = path.rfind('/');
  if (index != std::wstring::npos)
  {
    path = path.substr(index + 1);
  }

  auto module = runtime->LoadModule(path);
  if (module)
  {
    module->Relocate();
    *handle = module->handle();
    retval.val = 0;
    return true;
  }

  if (path.length() >= 5 && path.substr(path.length() - 5) == L".sprx")
  {
    module = runtime->LoadModule(path.substr(0, path.length() - 5) + L".prx");
    if (module)
    {
      module->Relocate();
      *handle = module->handle();
      retval.val = 0;
      return true;
    }
  }

  printf("LOAD PRX FAILED!\n");
  retval.val = -1;
  return false;
}

SYSCALL_IMPL(dynlib_do_copy_relocations)
{
  return true;
}

SYSCALL_IMPL(dynlib_get_proc_param, void** data_address, size_t* data_size)
{
  auto base_address = runtime->boot_module_->base_address();
  *data_address = base_address ? &base_address[runtime->boot_module_->sce_proc_param_address()] : nullptr;
  *data_size = runtime->boot_module_->sce_proc_param_size();
  return true;
}

SYSCALL_IMPL(dynlib_process_needed_and_relocate)
{
  bool success = runtime->LoadNeededModules() && runtime->SortModules() && runtime->RelocateModules();
  retval.val = success ? 0 : -1;
  return success;
}

SYSCALL_IMPL(mdbg_service, uint32_t op, void* arg2, void* arg3)
{
  if (op == 1)
  {
    return true;
  }

  assert_always();
  retval.val = -1;
  return false;
}

SYSCALL_IMPL(randomized_path, const char* set_path, char* path, size_t* path_length)
{
  if (set_path != nullptr)
  {
    retval.val = -1;
    return false;
  }

  *path_length = snprintf(path, *path_length, "uplift");
  return true;
}

SYSCALL_IMPL(workaround8849)
{
  return true;
}

struct dynlib_info_ex
{
  uint64_t struct_size;
  char name[256];
  uint32_t handle;
  uint16_t tls_index;
  uint16_t unknown_10E;
  void* tls_address;
  uint32_t tls_file_size;
  uint32_t tls_memory_size;
  uint32_t tls_offset;
  uint32_t tls_align;
  void* init_address;
  void* fini_address;
  uint64_t unknown_138;
  uint64_t unknown_140;
  void* eh_frame_header_buffer;
  void* eh_frame_data_buffer;
  uint32_t eh_frame_header_size;
  uint32_t eh_frame_data_size;
  void* text_address;
  uint32_t text_size;
  uint32_t text_flags;
  void* data_address;
  uint32_t data_size;
  uint32_t data_flags;
  uint8_t unknown_180[32];
  uint32_t unknown_1A0;
  int32_t ref_count;
};

SYSCALL_IMPL(dynlib_get_info_ex, uint32_t handle, void* arg2, void* vinfo)
{
  if (static_cast<dynlib_info_ex*>(vinfo)->struct_size != sizeof(dynlib_info_ex))
  {
    retval.err = SCERR::eINVAL;
    return false;
  }

  auto module = runtime->object_table()->LookupObject<Module>(handle).get();
  if (!module)
  {
    retval.err = SCERR::eSRCH;
    return false;
  }

  auto name = xe::to_string(module->name());
  auto index = name.rfind('.');
  if (index != std::string::npos)
  {
    name = name.substr(0, index);
  }

  auto base_address = module->base_address();
  auto program_info = module->program_info();
  auto dynamic_info = module->dynamic_info();

  dynlib_info_ex info = {};
  std::strncpy(info.name, name.c_str(), sizeof(info.name));
  info.handle = module->handle();
  info.struct_size = sizeof(dynlib_info_ex);
  info.tls_index = module->tls_index();
  info.tls_address = !program_info.tls_memory_size ? nullptr : &base_address[program_info.tls_address];
  info.tls_file_size = static_cast<uint32_t>(program_info.tls_file_size);
  info.tls_memory_size = static_cast<uint32_t>(program_info.tls_memory_size);
  info.tls_align = static_cast<uint32_t>(program_info.tls_align);
  info.init_address = !dynamic_info.has_init_offset ? nullptr : &base_address[dynamic_info.init_offset];
  info.fini_address = !dynamic_info.has_fini_offset ? nullptr : &base_address[dynamic_info.fini_offset];
  info.eh_frame_header_buffer = !program_info.eh_frame_address ? nullptr : &base_address[program_info.eh_frame_address];
  info.eh_frame_header_size = static_cast<uint32_t>(program_info.eh_frame_memory_size);
  info.eh_frame_data_buffer = module->eh_frame_data_buffer();
  info.eh_frame_data_size = static_cast<uint32_t>(module->eh_frame_data_size());
  info.text_address = module->text_address();
  info.text_size = static_cast<uint32_t>(module->text_size());
  info.text_flags = 1 | 4; // R+X
  info.data_address = module->data_address();
  info.data_size = static_cast<uint32_t>(module->data_size());
  info.data_flags = 1 | 2; // R+W
  info.unknown_1A0 = 0; // 2
  info.ref_count = module->pointer_ref_count();
  std::memcpy(vinfo, &info, sizeof(dynlib_info_ex));
  return true;
}

// arg1 removed after 1.76 sometime?
SYSCALL_IMPL(eport_create, /*const char* name,*/ uint32_t pid)
{
  printf("eport_create: %x\n", pid);

  if (pid != -1 && pid != 123)
  {
    retval.err = SCERR::eNOSYS;
    return false;
  }

  auto eport = object_ref<Eport>(new Eport(runtime)).get();
  SCERR result = SUCCESS; // Init?
  if (IS_ERROR(result))
  {
    eport->ReleaseHandle();
    retval.err = SCERR::eAGAIN;
    return false;
  }
  runtime->eport_ = eport;
  retval.err = SUCCESS;
  return true;
}

SYSCALL_IMPL(get_proc_type_info, void* vtype_info)
{
  struct
  {
    size_t struct_size;
    uint32_t budget;
    uint32_t flags;
  }
  type_info = { sizeof(type_info), 0, 0 };
  std::memcpy(vtype_info, &type_info, sizeof(type_info));
  retval.val = 0;
  return true;
}

SYSCALL_IMPL(thr_get_name, uint32_t id, char* name)
{
  snprintf(name, 31, "thread_%u", id);
  return true;
}

enum class ipmimgr_op : uint32_t
{
  CreateServer = 0,
  DestroyServer = 1,
  CreateClient = 2,
  DestroyClient = 3,
  CreateSession = 4,
  DestroySession = 5,
  Trace = 16,
  ReceivePacket = 513,
  __u514 = 514,
  __u529 = 529, // connect related
  __u530 = 530, // connect related
  __u531 = 531, // connect related
  __u546 = 546,
  __u547 = 547,
  __u561 = 561, // InvokeSyncMethod
  __u563 = 563,
  InvokeAsyncMethod = 577,
  TryGetResult = 579,
  GetMessage_ = 593,
  TryGetMessage = 594,
  SendMessage_ = 595,
  TrySendMessage = 596,
  EmptyMessageQueue = 597,
  __u609 = 609,
};

SYSCALL_IMPL(ipmimgr_call, uint32_t op, uint32_t handle, uint32_t* result, void* args_buffer, size_t args_size, uint64_t cookie)
{
  printf("ipmimgr_call: %u, %u, %p, %p, %I64x, %I64x\n", op, handle, result, args_buffer, args_size, cookie);

  if (args_size > 64)
  {
    *result = 0x800E0001;
    return false;
  }

  switch (static_cast<ipmimgr_op>(op))
  {
    case ipmimgr_op::CreateClient:
    {
      struct op_args
      {
        void* arg1;
        const char* name;
        void* arg3;
      };
      auto args = static_cast<op_args*>(args_buffer);
      printf("ipmimgr_call create: %s\n", args->name);
      auto ipmi_client = object_ref<IpmiClient>(new IpmiClient(runtime)).get();
      auto init_result = ipmi_client->Initialize(args->arg1, std::string(args->name), args->arg3);
      if (IS_ERROR(init_result))
      {
        ipmi_client->ReleaseHandle();
        retval.err = init_result;
        return false;
      }
      *result = ipmi_client->handle();
      return true;
    }

    case ipmimgr_op::DestroyClient:
    {
      assert_always();
      *result = 0;
      retval.val = 0;
      return true;
    }

    case ipmimgr_op::Trace:
    {
      if (!args_buffer || args_size < 64)
      {
        retval.err = SCERR::eINVAL;
        return false;
      }
      struct trace_args
      {
        uint32_t client_handle;
        uint32_t unknown_04;
        char name[25];
        uint32_t unknown_24;
        uint32_t unknown_28;
        uint32_t unknown_2C;
        uint32_t unknown_30;
        uint32_t unknown_34;
        uint32_t unknown_38;
        uint32_t unknown_3C;
      };
      static_assert_size(trace_args, 64);
      auto args = static_cast<trace_args*>(args_buffer);
      printf("ipmi trace(%u): client handle=%d %u name='%s' %u %u %u %u %u %u %u\n", handle, args->client_handle, args->unknown_04, args->name, args->unknown_24, args->unknown_28, args->unknown_2C, args->unknown_30, args->unknown_34, args->unknown_38, args->unknown_3C);
      *result = 0;
      return true;
    }

    case ipmimgr_op::__u529: // prepare connect?
    {
      struct op_data
      {
        uint32_t pid;
        uint32_t unknown_004;
        uint32_t unknown_008;
        uint32_t unknown_00C;
        uint64_t unknown_010;
        uint64_t unknown_018;
        uint32_t unknown_020;
        uint32_t unknown_024;
        uint64_t unknown_028;
        uint64_t unknown_030;
        uint32_t event_flag_count;
        uint32_t unknown_03C;
        uint32_t unknown_040;
        uint32_t unknown_044;
        uint64_t unknown_048[32]; // just to reduce the complexity of the struct for now
        uint64_t unknown_148;
        uint32_t unknown_150;
        uint32_t unknown_154;
        uint32_t unknown_158;
        uint32_t client_handle;
      };
      static_assert_size(op_data, 352);
      struct op_args
      {
        op_data* data;
        uint64_t arg2;
        size_t data_size;
        uint64_t arg4;
      };
      auto args = static_cast<op_args*>(args_buffer);
      auto ipmi_client = runtime->object_table()->LookupObject<IpmiClient>(static_cast<ObjectHandle>(handle)).get();
      if (!ipmi_client)
      {
        retval.err = SCERR::eNOENT;
        return false;
      }
      auto prepare_result = ipmi_client->PrepareConnect(args->data->event_flag_count);
      if (IS_ERROR(prepare_result))
      {
        retval.err = prepare_result;
        return false;
      }
      *result = 0;
      return true;
    }

    case ipmimgr_op::__u531: // connect?
    {
      struct op_args
      {
        uint64_t* session_key;
        uint32_t* unknown;
        uint32_t* session_id;
        uint32_t* result;
      };
      auto args = static_cast<op_args*>(args_buffer);
      auto ipmi_client = runtime->object_table()->LookupObject<IpmiClient>(static_cast<ObjectHandle>(handle)).get();
      if (!ipmi_client)
      {
        retval.err = SCERR::eNOENT;
        return false;
      }
      auto connect_result = ipmi_client->Connect(args->session_key, args->unknown, args->session_id, args->result);
      if (IS_ERROR(connect_result))
      {
        retval.err = connect_result;
        return false;
      }
      *result = 0;
      return true;
    }

    case ipmimgr_op::__u561:
    case ipmimgr_op::__u563:
    {
      *result = 0;
      return true;
    }
  }

  assert_always();
  retval.val = -1;
  return false;
}

SYSCALL_IMPL(utc_to_localtime, uint64_t arg1, uint64_t* arg2, void* arg3, uint32_t* arg4)
{
  if (arg2) *arg2 = arg1; // just for now
  if (arg3) std::memset(arg3, 0, 16);
  if (arg4) *arg4 = 0;
  return true;
}

SYSCALL_IMPL(dynlib_get_obj_member, uint32_t handle, uint8_t index, void** value)
{
  auto module = runtime->object_table()->LookupObject<Module>(handle).get();
  if (!module)
  {
    retval.err = SCERR::eSRCH;
    return false;
  }

  switch (index)
  {
    case 1:
    {
      auto dynamic_info = module->dynamic_info();
      *value = !dynamic_info.has_init_offset ? nullptr : &module->base_address()[dynamic_info.init_offset];
      break;
    }

    default:
    {
      retval.err = SCERR::eINVAL;
      return false;
    }
  }

  return true;
}

void uplift::get_syscall_table(SyscallEntry table[SyscallTableSize])
{
#define SYSCALL(x,y,...) { table[x].handler = SYSCALLS::y; table[x].name = #y; }
#include "syscall_table.inl"
#undef SYSCALL
}
