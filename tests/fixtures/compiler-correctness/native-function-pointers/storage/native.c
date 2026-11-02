#include <stdbool.h>
#include <stdint.h>

typedef int32_t (*Callback)(int32_t);

typedef struct FunctionTable {
    Callback plain;
    Callback leaf;
    Callback optional;
} FunctionTable;

static int32_t add_one(int32_t value) { return value + 1; }
static int32_t add_two(int32_t value) { return value + 2; }

FunctionTable pointer_table(void) {
    return (FunctionTable){add_one, add_two, 0};
}

bool pointer_check_table(FunctionTable value) {
    return value.plain == add_one && value.leaf == add_two && value.optional == 0 &&
           value.plain(40) == 41 && value.leaf(40) == 42;
}

bool pointer_check_leaf(Callback value) { return value == add_two; }
bool pointer_check_plain(Callback value) { return value == add_two; }
