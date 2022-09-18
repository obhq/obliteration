#include "Kyty/Core/MemoryAlloc.h"

#include "Kyty/Core/ArrayWrapper.h" // IWYU pragma: keep
#include "Kyty/Core/Common.h"
#include "Kyty/Core/DateTime.h" // IWYU pragma: keep
#include "Kyty/Core/Debug.h"    // IWYU pragma: keep
#include "Kyty/Core/Hashmap.h"  // IWYU pragma: keep
#include "Kyty/Sys/SysHeap.h"
#include "Kyty/Sys/SysSync.h"

#include <cstdlib>
#include <new>

namespace Kyty::Core {

#define MEM_ALLOC_ALIGNED

#ifdef MEM_ALLOC_ALIGNED
#if KYTY_PLATFORM == KYTY_PLATFORM_ANDROID
constexpr int MEM_ALLOC_ALIGN = 8;
#else
constexpr int MEM_ALLOC_ALIGN = 16;
#endif
#endif

#if KYTY_PLATFORM == KYTY_PLATFORM_WINDOWS && KYTY_BITNESS == 64
[[maybe_unused]] constexpr int STACK_CHECK_FROM = 5;
#elif KYTY_PLATFORM == KYTY_PLATFORM_ANDROID
[[maybe_unused]] constexpr int STACK_CHECK_FROM = 4;
#else
[[maybe_unused]] constexpr int STACK_CHECK_FROM = 2;
#endif

static SysCS*        g_mem_cs          = nullptr;
static sys_heap_id_t g_default_heap    = nullptr; // HeapCreate(HEAP_NO_SERIALIZE, 0, 0)

class MemLock
{
public:
	MemLock()
	{
		g_mem_cs->Enter();
	}

	~MemLock()
	{
		g_mem_cs->Leave();
	}

	KYTY_CLASS_NO_COPY(MemLock);
};

#if KYTY_COMPILER == KYTY_COMPILER_MSVC
#pragma code_seg(push)
#endif

#if KYTY_COMPILER == KYTY_COMPILER_MSVC
#pragma code_seg(".mem_a")
#endif

#if KYTY_COMPILER == KYTY_COMPILER_MSVC
#pragma code_seg(".mem_b")
#endif

void* mem_alloc_check_alignment(void* ptr)
{
#ifdef MEM_ALLOC_ALIGNED
	if ((reinterpret_cast<uintptr_t>(ptr) & static_cast<uintptr_t>(MEM_ALLOC_ALIGN - 1)) != 0u)
	{
		EXIT("mem alloc not aligned!\n");
	}
#endif

	return ptr;
}

void* mem_alloc(size_t size)
{
	if (size == 0)
	{
		EXIT("size == 0\n");
	}

	mem_init();
	MemLock lock;

	void* ptr  = sys_heap_alloc(g_default_heap, size);

	if (ptr == nullptr)
	{
		EXIT("mem_alloc(): can't alloc %" PRIu64 " bytes\n", uint64_t(size));
	}

	return mem_alloc_check_alignment(ptr);
}

void* mem_realloc(void* ptr, size_t size)
{
	EXIT_IF(size == 0);

	mem_init();
	MemLock lock;

	void* ptr2 = sys_heap_realloc(g_default_heap, ptr, size);

	if (ptr2 == nullptr)
	{
		EXIT("mem_realloc(): can't alloc %" PRIu64 " bytes\n", uint64_t(size));
	}

	// g_mem_cs->Leave();

	return mem_alloc_check_alignment(ptr2);
}

void mem_free(void* ptr)
{

	EXIT_IF(!g_mem_initialized);

	MemLock lock;

	sys_heap_free(g_default_heap, ptr);
}

bool mem_check([[maybe_unused]] const void* ptr)
{
	return true;
}

void mem_get_stat(MemStats* s)
{
		s->total_allocated = 0;
		s->blocks_num      = 0;
}

int mem_new_state()
{
		return 0;
}

void mem_print(int from_state)
{
}

} // namespace Kyty::Core

#ifndef KYTY_SHARED_DLL
void* operator new(size_t size)
{
	return Kyty::Core::mem_alloc(size);
}

void* operator new(std::size_t size, const std::nothrow_t& /*nothrow_value*/) noexcept
{
	return Kyty::Core::mem_alloc(size);
}

void* operator new[](size_t size)
{
	return Kyty::Core::mem_alloc(size);
}

void* operator new[](std::size_t size, const std::nothrow_t& /*nothrow_value*/) noexcept
{
	return Kyty::Core::mem_alloc(size);
}

void operator delete(void* block) noexcept
{
	Kyty::Core::mem_free(block);
}

void operator delete[](void* block) noexcept
{
	Kyty::Core::mem_free(block);
}
#else
//#error "haha"
#endif

extern "C" {
void* mem_alloc_c(size_t size)
{
	return Kyty::Core::mem_alloc(size);
}

void* mem_realloc_c(void* ptr, size_t size)
{
	return Kyty::Core::mem_realloc(ptr, size);
}

void mem_free_c(void* ptr)
{
	Kyty::Core::mem_free(ptr);
}
}

namespace Kyty::Core {

bool mem_tracker_enabled()
{
	return false;
}

void mem_tracker_enable()
{
}

void mem_tracker_disable()
{
}

#if KYTY_COMPILER == KYTY_COMPILER_MSVC
#pragma code_seg(".mem_c")
#endif

#if KYTY_COMPILER == KYTY_COMPILER_MSVC
#pragma code_seg(pop)
#endif

} // namespace Kyty::Core
