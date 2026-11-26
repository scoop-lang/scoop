#define _GNU_SOURCE 1
#include "../../scoop.platform/native/files.h"
#include <sys/stat.h>

typedef struct {
    ScoopPlatformFileMetadata basic;
    uint32_t user_id, group_id;
    uint64_t device_id, inode, link_count;
} UnixMetadata;

UnixMetadata scoop_os_nix_stat(const char *path, bool follow) {
    struct stat data;
    if ((follow ? stat(path, &data) : lstat(path, &data)) < 0)
        return (UnixMetadata){.basic = {.kind = -1}};
#ifdef __APPLE__
    uint64_t device = (uint32_t)data.st_dev;
#else
    uint64_t device = data.st_dev;
#endif
    return (UnixMetadata){scoop_platform_metadata_from_stat(&data),
                          data.st_uid,
                          data.st_gid,
                          device,
                          data.st_ino,
                          data.st_nlink};
}

int32_t scoop_os_nix_mkfifo(const char *path, int32_t mode) { return mkfifo(path, (mode_t)mode); }
