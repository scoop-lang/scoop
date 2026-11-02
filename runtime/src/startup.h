#ifndef SCOOP_RT_STARTUP_H
#define SCOOP_RT_STARTUP_H

#include "scoop_app.h"
#include "scoop_runtime_metadata_v1.h"

/* One complete program lifetime. The pointer array and records belong to the
 * loaded main image; no managed code may run before this call registers them.
 */
int scoop_rt_run_program(const ScoopImageDescriptorV1 *const *images, uint64_t image_count,
                         const ScoopRootEntryDescriptorV1 *root_entry, int32_t argc,
                         const char *const *argv, uint32_t enable_stack_trace);

/* A persistent runtime owned by one loaded native-app module. */
int32_t scoop_rt_init_native_app(const ScoopImageDescriptorV1 *const *images, uint64_t image_count,
                                 const ScoopNativeAppEntryDescriptorV1 *root_entry,
                                 const void *host_api, void *out_app_api, ScoopAppError *out_error,
                                 uint32_t enable_stack_trace);

#if defined(_WIN32)
int scoop_rt_run_program_utf16(const ScoopImageDescriptorV1 *const *images, uint64_t image_count,
                               const ScoopRootEntryDescriptorV1 *root_entry, int32_t argc,
                               const uint16_t *const *argv, uint32_t enable_stack_trace);
#endif

#endif
