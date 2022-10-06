#pragma once

#ifdef _WIN32
#define S(x) L##x
#else
#define S(x) x
#endif
