#include "stdafx.h"
#include "match.hpp"

bool match_buffer(void* datBuf, size_t datLen, void* patBuf, size_t patLen, void** result)
{
  if (!result)
  {
    return false;
  }

  *result = nullptr;
  if (!datBuf || !datLen || !patBuf || !patLen)
  {
    return false;
  }

  auto datBytes = static_cast<unsigned char*>(datBuf);
  auto patWords = static_cast<unsigned short*>(patBuf);
  for (size_t i = 0; i + patLen <= datLen; i++)
  {
    bool ismatch = true;
    for (size_t j = 0; ismatch && j < patLen; j++)
    {
      auto pat = patWords[j];
      auto mask = (unsigned char)(~(pat >> 8));
      if (mask == 0) continue;
      auto val = (unsigned char)(pat);
      ismatch = (val & mask) == (datBytes[i + j] & mask);
    }

    if (ismatch)
    {
      *result = &datBytes[i];
      return true;
    }
  }
  return false;
}
