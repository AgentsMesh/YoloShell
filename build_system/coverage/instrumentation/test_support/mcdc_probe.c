#include <stdbool.h>
#include <stdint.h>
#include <stdatomic.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

// Rust 1.93 condition coverage is not MC/DC. Clang supplies a real bitmap.
enum { EXPECTED_ARGUMENTS = 2, MODE_ARGUMENT = 1, CALLS = 8, TRUE_RESULTS = 3 };
enum { SUCCESS = 0, FAILURE = 1, A_MASK = 4, B_MASK = 2, C_MASK = 1 };
#define INITIAL_MARKER UINT64_C(0x123456789abcdef0)
static const uint64_t ADVANCED_MARKER = UINT64_C(0x123456789abcdef1);
static const char NORMAL_MODE[] = "normal";
static const char BLOCKED_MODE[] = "blocked";
static const char READY[] = "profile-ready\n";

__attribute__((used, section("__DATA,__yolo_unknown")))
static _Atomic(uint64_t) unknown_data = INITIAL_MARKER;

__attribute__((noinline))
bool yolo_coverage_layout_decision(bool a, bool b, bool c) {
    return a && (b || c);
}

int main(int argc, char **argv) {
    if (argc != EXPECTED_ARGUMENTS || atomic_load(&unknown_data) != INITIAL_MARKER)
        return FAILURE;
    atomic_store(&unknown_data, ADVANCED_MARKER);
    unsigned accepted = 0;
    for (unsigned input = 0; input < CALLS; ++input)
        accepted += yolo_coverage_layout_decision(input & A_MASK, input & B_MASK, input & C_MASK);
    if (accepted != TRUE_RESULTS || atomic_load(&unknown_data) != ADVANCED_MARKER)
        return FAILURE;
    if (fputs(READY, stdout) == EOF || fflush(stdout) != SUCCESS)
        return FAILURE;
    if (strcmp(argv[MODE_ARGUMENT], BLOCKED_MODE) == SUCCESS) {
        for (;;) pause();
    }
    return strcmp(argv[MODE_ARGUMENT], NORMAL_MODE) == SUCCESS ? SUCCESS : FAILURE;
}
