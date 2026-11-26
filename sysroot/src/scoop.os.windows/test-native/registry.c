#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdint.h>

uint64_t scoop_test_registry_id(void) {
    return ((uint64_t)GetCurrentProcessId() << 32) | GetTickCount();
}

int32_t scoop_test_registry_create(const WCHAR *path) {
    HKEY key;
    DWORD disposition;
    LSTATUS status = RegCreateKeyExW(HKEY_CURRENT_USER, path, 0, NULL, REG_OPTION_VOLATILE,
                                     KEY_SET_VALUE, NULL, &key, &disposition);
    if (status != ERROR_SUCCESS)
        return status;
    if (disposition != REG_CREATED_NEW_KEY) {
        RegCloseKey(key);
        return ERROR_ALREADY_EXISTS;
    }
    DWORD number = 0xf1234567u;
    const WCHAR text[] = {0x96ea, 0xd800, 0, L'x', 0, 0};
    const WCHAR plain[] = L"%TEMP%";
    status = RegSetValueExW(key, L"number", 0, REG_DWORD, (const BYTE *)&number, sizeof(number));
    if (!status)
        status = RegSetValueExW(key, NULL, 0, REG_SZ, (const BYTE *)text, sizeof(text));
    if (!status)
        status = RegSetValueExW(key, L"empty", 0, REG_SZ, (const BYTE *)L"", sizeof(WCHAR));
    if (!status)
        status = RegSetValueExW(key, L"plain", 0, REG_SZ, (const BYTE *)plain, sizeof(plain));
    if (!status)
        status = RegSetValueExW(key, L"expand", 0, REG_EXPAND_SZ, (const BYTE *)plain, sizeof(plain));
    RegCloseKey(key);
    if (status)
        RegDeleteKeyW(HKEY_CURRENT_USER, path);
    return status;
}

int32_t scoop_test_registry_delete(const WCHAR *path) { return RegDeleteKeyW(HKEY_CURRENT_USER, path); }
