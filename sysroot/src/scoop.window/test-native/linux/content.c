#define _GNU_SOURCE
#include <limits.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>
#include <wayland-client.h>

typedef struct Content Content;
typedef struct Buffer {
  struct wl_buffer *buffer;
  Content *content;
  struct Buffer *next;
} Buffer;
struct Content {
  struct wl_display *display;
  struct wl_surface *surface;
  struct wl_registry *registry;
  struct wl_shm *shm;
  Buffer *buffers;
};

static void global(void *data, struct wl_registry *registry, uint32_t name,
                   const char *interface, uint32_t version) {
  Content *content = data;
  if (!strcmp(interface, "wl_shm"))
    content->shm = wl_registry_bind(registry, name, &wl_shm_interface, 1);
}
static void removed(void *data, struct wl_registry *registry, uint32_t name) {}
static const struct wl_registry_listener globals = {global, removed};

void scoop_test_wayland_content_destroy(Content *content) {
  while (content->buffers) {
    Buffer *buffer = content->buffers;
    content->buffers = buffer->next;
    wl_buffer_destroy(buffer->buffer);
    free(buffer);
  }
  if (content->shm)
    wl_shm_destroy(content->shm);
  wl_registry_destroy(content->registry);
  free(content);
}

void *scoop_test_wayland_content_create(struct wl_display *display,
                                        struct wl_surface *surface) {
  Content *content = calloc(1, sizeof(*content));
  if (!content)
    return NULL;
  content->display = display;
  content->surface = surface;
  content->registry = wl_display_get_registry(display);
  wl_registry_add_listener(content->registry, &globals, content);
  if (wl_display_roundtrip(display) < 0 || !content->shm) {
    scoop_test_wayland_content_destroy(content);
    return NULL;
  }
  return content;
}

static void released(void *data, struct wl_buffer *object) {
  Buffer *buffer = data;
  Buffer **link = &buffer->content->buffers;
  while (*link != buffer)
    link = &(*link)->next;
  *link = buffer->next;
  wl_buffer_destroy(object);
  free(buffer);
}
static const struct wl_buffer_listener buffers = {released};

int32_t scoop_test_wayland_content_present(Content *content, int32_t width,
                                           int32_t height) {
  if (width <= 0 || height <= 0 || (int64_t)width * height > INT_MAX / 4)
    return -1;
  int size = width * height * 4;
  int fd = memfd_create("scoop-window-test", MFD_CLOEXEC);
  if (fd < 0)
    return -1;
  if (ftruncate(fd, size)) {
    close(fd);
    return -1;
  }
  uint32_t *pixels =
      mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
  if (pixels == MAP_FAILED) {
    close(fd);
    return -1;
  }
  for (int index = 0; index < width * height; ++index)
    pixels[index] = 0xff276749;
  munmap(pixels, size);
  Buffer *buffer = calloc(1, sizeof(*buffer));
  if (!buffer) {
    close(fd);
    return -1;
  }
  struct wl_shm_pool *pool = wl_shm_create_pool(content->shm, fd, size);
  buffer->buffer = wl_shm_pool_create_buffer(pool, 0, width, height, width * 4,
                                             WL_SHM_FORMAT_ARGB8888);
  wl_shm_pool_destroy(pool);
  close(fd);
  buffer->content = content;
  buffer->next = content->buffers;
  content->buffers = buffer;
  wl_buffer_add_listener(buffer->buffer, &buffers, buffer);
  wl_surface_attach(content->surface, buffer->buffer, 0, 0);
  wl_surface_damage(content->surface, 0, 0, INT_MAX, INT_MAX);
  wl_surface_commit(content->surface);
  return 0;
}
