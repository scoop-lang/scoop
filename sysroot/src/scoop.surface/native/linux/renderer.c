#include <EGL/egl.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <wayland-client.h>
#include <wayland-egl.h>
#include <xcb/xcb.h>

typedef int32_t (*SSCallback)(void *, int32_t);
typedef struct {
  void *pointer;
  int32_t status;
} SSAcquired;
typedef struct {
  EGLDisplay display;
  EGLConfig config;
  EGLContext egl_context;
  EGLSurface surface;
  struct wl_surface *wayland_surface;
  struct wl_egl_window *wayland_window;
  struct wl_callback *frame_callback;
  xcb_connection_t *connection;
  xcb_window_t window;
  SSCallback callback;
  void *context;
  int width, height;
  double scale;
  uint64_t generation;
  bool wanted, notified, closed, acquired;
} SSSurface;
typedef struct {
  SSSurface *surface;
  uint64_t generation;
} SSFrame;

static void notify(SSSurface *surface) {
  if (surface->closed || !surface->wanted || surface->notified ||
      surface->frame_callback || surface->width <= 0 || surface->height <= 0)
    return;
  surface->notified = true;
  if (!surface->callback(surface->context, 1))
    surface->wanted = false;
}
static void cancel_callback(SSSurface *surface) {
  if (surface->frame_callback) {
    wl_callback_destroy(surface->frame_callback);
    surface->frame_callback = NULL;
  }
  surface->notified = false;
}
static void frame_done(void *data, struct wl_callback *callback,
                       uint32_t time) {
  SSSurface *surface = data;
  wl_callback_destroy(callback);
  surface->frame_callback = NULL;
  notify(surface);
}
static const struct wl_callback_listener frames = {frame_done};

void *scoop_surface_gles_create(void *display, void *config, void *egl_context,
                                void *connection, uint32_t window,
                                void *wayland_surface, SSCallback callback,
                                void *context) {
  SSSurface *surface = calloc(1, sizeof(*surface));
  if (!surface)
    return NULL;
  surface->display = display;
  surface->config = config;
  surface->egl_context = egl_context;
  surface->connection = connection;
  surface->window = window;
  surface->wayland_surface = wayland_surface;
  surface->callback = callback;
  surface->context = context;
  return surface;
}

static int create_surface(SSSurface *surface) {
  void *native_window = &surface->window;
  if (surface->wayland_surface) {
    surface->wayland_window = wl_egl_window_create(
        surface->wayland_surface, surface->width, surface->height);
    if (!surface->wayland_window)
      return -1;
    native_window = surface->wayland_window;
  }
  surface->surface = eglCreatePlatformWindowSurface(
      surface->display, surface->config, native_window, NULL);
  if (surface->surface == EGL_NO_SURFACE) {
    if (surface->wayland_window)
      wl_egl_window_destroy(surface->wayland_window);
    surface->wayland_window = NULL;
    return -1;
  }
  if (!eglMakeCurrent(surface->display, surface->surface, surface->surface,
                      surface->egl_context) ||
      !eglSwapInterval(surface->display, 1)) {
    eglMakeCurrent(surface->display, EGL_NO_SURFACE, EGL_NO_SURFACE,
                   surface->egl_context);
    eglDestroySurface(surface->display, surface->surface);
    surface->surface = EGL_NO_SURFACE;
    if (surface->wayland_window)
      wl_egl_window_destroy(surface->wayland_window);
    surface->wayland_window = NULL;
    return -1;
  }
  return 1;
}

static bool destroy_surface(SSSurface *surface) {
  if (surface->surface == EGL_NO_SURFACE)
    return true;
  if (eglGetCurrentSurface(EGL_DRAW) == surface->surface &&
      !eglMakeCurrent(surface->display, EGL_NO_SURFACE, EGL_NO_SURFACE,
                      surface->egl_context))
    return false;
  if (!eglDestroySurface(surface->display, surface->surface))
    return false;
  surface->surface = EGL_NO_SURFACE;
  if (surface->wayland_window)
    wl_egl_window_destroy(surface->wayland_window);
  surface->wayland_window = NULL;
  return true;
}

int32_t scoop_surface_gles_configure(SSSurface *surface, int32_t width,
                                     int32_t height, double scale) {
  if (surface->closed || width < 0 || height < 0 || scale <= 0)
    return -1;
  if (surface->width != width || surface->height != height ||
      surface->scale != scale) {
    if ((surface->width != width || surface->height != height) &&
        !destroy_surface(surface))
      return -1;
    surface->width = width;
    surface->height = height;
    surface->scale = scale;
    ++surface->generation;
    cancel_callback(surface);
  }
  if (width > 0 && height > 0 && surface->surface == EGL_NO_SURFACE &&
      create_surface(surface) != 1)
    return -1;
  notify(surface);
  return 1;
}
int32_t scoop_surface_gles_demand(SSSurface *surface, bool required) {
  surface->wanted = required;
  if (!required)
    cancel_callback(surface);
  else
    notify(surface);
  return 1;
}

static int current(SSSurface *surface) {
  if (surface->closed || surface->surface == EGL_NO_SURFACE ||
      surface->width <= 0 || surface->height <= 0)
    return 0;
  if (surface->connection) {
    xcb_get_geometry_reply_t *geometry = xcb_get_geometry_reply(
        surface->connection,
        xcb_get_geometry(surface->connection, surface->window), NULL);
    xcb_get_window_attributes_reply_t *attributes =
        xcb_get_window_attributes_reply(
            surface->connection,
            xcb_get_window_attributes(surface->connection, surface->window),
            NULL);
    int status = geometry && attributes
                     ? geometry->width == surface->width &&
                           geometry->height == surface->height &&
                           attributes->map_state == XCB_MAP_STATE_VIEWABLE
                     : -1;
    free(geometry);
    free(attributes);
    return status;
  }
  return 1;
}
SSAcquired scoop_surface_gles_acquire(SSSurface *surface) {
  if (surface->acquired)
    return (SSAcquired){NULL, -1};
  surface->notified = false;
  if (surface->frame_callback)
    return (SSAcquired){NULL, 0};
  int status = current(surface);
  if (status != 1)
    return (SSAcquired){NULL, status};
  SSFrame *frame = malloc(sizeof(*frame));
  if (!frame)
    return (SSAcquired){NULL, -1};
  *frame = (SSFrame){surface, surface->generation};
  surface->acquired = true;
  return (SSAcquired){frame, 1};
}
void *scoop_surface_gles_frame_surface(SSFrame *frame) {
  return frame->surface->surface;
}
int32_t scoop_surface_gles_frame_current(SSFrame *frame) {
  return frame->generation == frame->surface->generation
             ? current(frame->surface)
             : 0;
}
int32_t scoop_surface_gles_present(SSFrame *frame) {
  SSSurface *surface = frame->surface;
  if (frame->generation != surface->generation || surface->closed)
    return 0;
  if (surface->wayland_surface && surface->wanted && !surface->frame_callback) {
    surface->frame_callback = wl_surface_frame(surface->wayland_surface);
    if (!surface->frame_callback)
      return -1;
    wl_callback_add_listener(surface->frame_callback, &frames, surface);
  }
  if (!eglMakeCurrent(surface->display, surface->surface, surface->surface,
                      surface->egl_context) ||
      !eglSwapBuffers(surface->display, surface->surface)) {
    cancel_callback(surface);
    return -1;
  }
  if (!surface->wayland_surface)
    notify(surface);
  return 1;
}
void scoop_surface_gles_frame_close(SSFrame *frame) {
  frame->surface->acquired = false;
  free(frame);
}
int32_t scoop_surface_gles_close(SSSurface *surface) {
  surface->closed = true;
  surface->wanted = false;
  cancel_callback(surface);
  bool ok = destroy_surface(surface);
  free(surface);
  return ok ? 1 : -1;
}
