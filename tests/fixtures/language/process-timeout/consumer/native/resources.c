#include <stdint.h>
#include <stdlib.h>

int32_t process_test_handles(void);
int32_t process_test_threads(void);

static void check(int success) {
    if (!success)
        abort();
}

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#ifdef _MSC_VER
#include <Windows.h>
#include <TlHelp32.h>
#else
#include <windows.h>
#include <tlhelp32.h>
#endif

int32_t process_test_handles(void) {
    DWORD count = 0;
    check(GetProcessHandleCount(GetCurrentProcess(), &count));
    return (int32_t)count;
}

int32_t process_test_threads(void) {
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    THREADENTRY32 entry = {0};
    int32_t count = 0;
    check(snapshot != INVALID_HANDLE_VALUE);
    entry.dwSize = sizeof(entry);
    if (Thread32First(snapshot, &entry)) {
        do {
            if (entry.th32OwnerProcessID == GetCurrentProcessId())
                ++count;
        } while (Thread32Next(snapshot, &entry));
    }
    check(CloseHandle(snapshot));
    return count;
}
#else
#include <dirent.h>
#include <string.h>

int32_t process_test_handles(void) {
    DIR *directory = opendir("/dev/fd");
    check(directory != NULL);
    int32_t count = 0;
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (strcmp(entry->d_name, ".") != 0 && strcmp(entry->d_name, "..") != 0)
            ++count;
    }
    check(closedir(directory) == 0);
    return count;
}

#ifdef __APPLE__
#include <mach/mach.h>

int32_t process_test_threads(void) {
    thread_act_array_t threads;
    mach_msg_type_number_t count;
    check(task_threads(mach_task_self(), &threads, &count) == KERN_SUCCESS);
    for (mach_msg_type_number_t index = 0; index < count; ++index)
        check(mach_port_deallocate(mach_task_self(), threads[index]) == KERN_SUCCESS);
    check(vm_deallocate(mach_task_self(), (vm_address_t)threads,
                         count * sizeof(thread_t)) == KERN_SUCCESS);
    return (int32_t)count;
}
#else
int32_t process_test_threads(void) {
    DIR *directory = opendir("/proc/self/task");
    check(directory != NULL);
    int32_t count = 0;
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (strcmp(entry->d_name, ".") != 0 && strcmp(entry->d_name, "..") != 0)
            ++count;
    }
    check(closedir(directory) == 0);
    return count;
}
#endif
#endif
