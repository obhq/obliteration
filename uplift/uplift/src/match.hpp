#pragma once

#define MATCH_MASK(x, y) ((unsigned short)(((unsigned short)((unsigned char)(~(unsigned char)x)) << 8) | ((unsigned char)y)))
#define MATCH_ANY MATCH_MASK(0, 0xCC)

bool match_buffer(void* datBuf, size_t datLen, void* patBuf, size_t patLen, void** result);
