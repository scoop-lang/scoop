#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct {
    uint32_t value;
    int32_t status;
} RegistryDword;

typedef struct {
    WCHAR *data;
    int64_t length;
    int32_t status;
} RegistryString;

static HKEY root_key(int32_t hive) { return hive == 0 ? HKEY_CURRENT_USER : HKEY_LOCAL_MACHINE; }

RegistryDword scoop_os_windows_registry_dword(int32_t hive, const WCHAR *key, const WCHAR *name) {
    DWORD value = 0, bytes = sizeof(value);
    LSTATUS status = RegGetValueW(root_key(hive), key, name, RRF_RT_REG_DWORD, NULL, &value, &bytes);
    if (status == ERROR_SUCCESS && bytes != sizeof(value))
        status = ERROR_INVALID_DATA;
    return (RegistryDword){value, status};
}

RegistryString scoop_os_windows_registry_string(int32_t hive, const WCHAR *key, const WCHAR *name) {
    const DWORD flags = RRF_RT_REG_SZ | RRF_NOEXPAND;
    HKEY root = root_key(hive);
    for (;;) {
        DWORD bytes = 0;
        LSTATUS status = RegGetValueW(root, key, name, flags, NULL, NULL, &bytes);
        if (status != ERROR_SUCCESS)
            return (RegistryString){.status = status};
        // A stored REG_SZ may lack a terminator; RegGetValueW supplies it on the data read.
        DWORD capacity = bytes <= MAXDWORD - sizeof(WCHAR) ? bytes + sizeof(WCHAR) : MAXDWORD;
        WCHAR *data = malloc(capacity);
        if (!data)
            return (RegistryString){.status = ERROR_NOT_ENOUGH_MEMORY};
        bytes = capacity;
        status = RegGetValueW(root, key, name, flags, NULL, data, &bytes);
        if (status == ERROR_SUCCESS) {
            if (bytes % sizeof(WCHAR) == 0) {
                DWORD count = bytes / sizeof(WCHAR);
                if (count && data[count - 1] == L'\0')
                    --count;
                return (RegistryString){data, count, ERROR_SUCCESS};
            }
            status = ERROR_INVALID_DATA;
        }
        free(data);
        if (status != ERROR_MORE_DATA || capacity == MAXDWORD)
            return (RegistryString){.status = status};
    }
}

void scoop_os_windows_registry_free(WCHAR *data) { free(data); }
