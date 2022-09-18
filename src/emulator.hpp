// This file is an interface to emulator/src/lib.rs.
#pragma once

#include <cinttypes>

extern "C" {
    void *emulator_init(char **error);
    void emulator_term(void *inst);

    void start_game(const std::uint16_t *dir, std::uintptr_t len);
}
