#include <stdio.h> // for debug print

#ifdef _WIN32
	#include <winsock2.h>
	#include <windows.h>
	#include <malloc.h>
#elif defined(__GNUC__)
	#include <unistd.h>
	#include <sys/mman.h>
	#include <stdlib.h>
#endif

#include <xbyak/xbyak.h>

#ifdef XBYAK_USE_MMAP_ALLOCATOR
uint8 *Xbyak::MmapAllocator::alloc(size_t size)
{
	const size_t alignedSizeM1 = inner::ALIGN_PAGE_SIZE - 1;
	size = (size + alignedSizeM1) & ~alignedSizeM1;
#ifdef MAP_ANONYMOUS
	const int mode = MAP_PRIVATE | MAP_ANONYMOUS;
#elif defined(MAP_ANON)
	const int mode = MAP_PRIVATE | MAP_ANON;
#else
	#error "not supported"
#endif
	void *p = mmap(NULL, size, PROT_READ | PROT_WRITE, mode, -1, 0);
	if (p == MAP_FAILED) throw Error(ERR_CANT_ALLOC);
	assert(p);
	sizeList_[(uintptr_t)p] = size;
	return (uint8*)p;
}
void Xbyak::MmapAllocator::free(uint8 *p)
{
	if (p == 0) return;
	SizeList::iterator i = sizeList_.find((uintptr_t)p);
	if (i == sizeList_.end()) throw Error(ERR_BAD_PARAMETER);
	if (munmap((void*)i->first, i->second) < 0) throw Error(ERR_MUNMAP);
	sizeList_.erase(i);
}
#endif

bool Xbyak::CodeArray::protect(const void *addr, size_t size, int protectMode)
{
#if defined(_WIN32)
	const DWORD c_rw = PAGE_READWRITE;
	const DWORD c_rwe = PAGE_EXECUTE_READWRITE;
	const DWORD c_re = PAGE_EXECUTE_READ;
	DWORD mode;
#else
	const int c_rw = PROT_READ | PROT_WRITE;
	const int c_rwe = PROT_READ | PROT_WRITE | PROT_EXEC;
	const int c_re = PROT_READ | PROT_EXEC;
	int mode;
#endif
	switch (protectMode) {
	case PROTECT_RW: mode = c_rw; break;
	case PROTECT_RWE: mode = c_rwe; break;
	case PROTECT_RE: mode = c_re; break;
	default:
		return false;
	}
#if defined(_WIN32)
	DWORD oldProtect;
	return VirtualProtect(const_cast<void*>(addr), size, mode, &oldProtect) != 0;
#elif defined(__GNUC__)
	size_t pageSize = sysconf(_SC_PAGESIZE);
	size_t iaddr = reinterpret_cast<size_t>(addr);
	size_t roundAddr = iaddr & ~(pageSize - static_cast<size_t>(1));
#ifndef NDEBUG
	if (pageSize != 4096) fprintf(stderr, "large page(%zd) is used. not tested enough.\n", pageSize);
#endif
	return mprotect(reinterpret_cast<void*>(roundAddr), size + (iaddr - roundAddr), mode) == 0;
#else
	return true;
#endif
}
