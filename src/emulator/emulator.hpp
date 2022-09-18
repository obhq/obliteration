#pragma once

#include <cinttypes>

extern "C" {
    void start_game(const std::uint16_t *dir, std::uintptr_t len);
}
