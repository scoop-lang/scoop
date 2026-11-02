#include "scoop_app.h"

int main() {
    ScoopAppError error = {reinterpret_cast<const uint8_t *>("old"), 3};
    int32_t status = scoop_app_init(nullptr, nullptr, &error);
    return status == SCOOP_APP_INIT_INVALID_ARGUMENT && error.bytes == nullptr && error.length == 0
               ? 0
               : 1;
}
