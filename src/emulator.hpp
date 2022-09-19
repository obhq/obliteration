// This file is an interface to emulator/src/lib.rs.
#pragma once

typedef struct emulator *emulator_t;

struct emulator_config {
};

extern "C" {
    emulator_t emulator_init(char **error);
    void emulator_term(emulator_t e);

    char *emulator_start(emulator_t e, const emulator_config *conf);
    int emulator_running(emulator_t e);
}
