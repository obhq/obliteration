#pragma once

#ifdef _WIN32
#define STR(x) L##x
#else
#define STR(x) x
#endif
