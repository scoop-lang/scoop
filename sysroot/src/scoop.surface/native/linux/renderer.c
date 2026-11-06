#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_14
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef int32_t (*SSCallback)(void *, int32_t);
typedef struct {
  void *pointer;
  int32_t status;
} SSAcquired;
typedef struct {
  GtkWidget *view;
  GtkGLArea *area;
  GdkGLContext *gl;
  SSCallback callback;
  void *context;
  guint tick;
  int width, height;
  double scale;
  uint64_t generation;
  bool wanted, notified, closed, acquired, ready, presented;
} SSSurface;
typedef struct {
  SSSurface *surface;
  uint64_t generation;
} SSFrame;

static int current(SSSurface *surface) {
  if (surface->closed || surface->width <= 0 || surface->height <= 0 ||
      !gtk_widget_get_mapped(GTK_WIDGET(surface->area)))
    return 0;
  if (gtk_gl_area_get_error(surface->area))
    return -1;
  int scale = gtk_widget_get_scale_factor(GTK_WIDGET(surface->area));
  return gtk_widget_get_width(GTK_WIDGET(surface->area)) * scale ==
             surface->width &&
         gtk_widget_get_height(GTK_WIDGET(surface->area)) * scale ==
             surface->height;
}
static void notify(SSSurface *surface) {
  if (surface->closed || !surface->wanted || surface->notified ||
      surface->acquired || !surface->ready || surface->presented ||
      current(surface) == 0)
    return;
  surface->notified = true;
  if (!surface->callback(surface->context, 1))
    surface->wanted = false;
}
static GdkGLContext *create_context(GtkGLArea *area, gpointer data) {
  return g_object_ref(((SSSurface *)data)->gl);
}
static gboolean render(GtkGLArea *area, GdkGLContext *context, gpointer data) {
  SSSurface *surface = data;
  surface->presented = false;
  surface->ready = true;
  notify(surface);
  return TRUE;
}
static void resize(GtkGLArea *area, int width, int height, gpointer data) {
  SSSurface *surface = data;
  ++surface->generation;
  surface->notified = false;
  surface->ready = false;
}
static gboolean tick(GtkWidget *widget, GdkFrameClock *clock, gpointer data) {
  SSSurface *surface = data;
  notify(surface);
  if (!surface->wanted || surface->closed) {
    surface->tick = 0;
    return G_SOURCE_REMOVE;
  }
  return G_SOURCE_CONTINUE;
}
void *scoop_surface_gtk_display(void *view) {
  return gtk_widget_get_display(GTK_WIDGET(view));
}
void *scoop_surface_gles_create(void *view, void *gl_context,
                                SSCallback callback, void *context) {
  if (gtk_widget_get_first_child(GTK_WIDGET(view)))
    return NULL;
  SSSurface *surface = calloc(1, sizeof(*surface));
  if (!surface)
    return NULL;
  surface->view = view;
  surface->gl = g_object_ref(gl_context);
  surface->callback = callback;
  surface->context = context;
  surface->area = GTK_GL_AREA(gtk_gl_area_new());
  gtk_gl_area_set_allowed_apis(surface->area, GDK_GL_API_GLES);
  gtk_gl_area_set_required_version(surface->area, 3, 0);
  gtk_gl_area_set_has_stencil_buffer(surface->area, TRUE);
  gtk_gl_area_set_auto_render(surface->area, FALSE);
  g_signal_connect(surface->area, "create-context", G_CALLBACK(create_context),
                   surface);
  g_signal_connect(surface->area, "render", G_CALLBACK(render), surface);
  g_signal_connect(surface->area, "resize", G_CALLBACK(resize), surface);
  gtk_widget_set_parent(GTK_WIDGET(surface->area), surface->view);
  g_object_ref(surface->area);
  gtk_widget_queue_resize(surface->view);
  return surface;
}
int32_t scoop_surface_gles_configure(SSSurface *surface, int32_t width,
                                     int32_t height, double scale) {
  if (surface->closed || width < 0 || height < 0 || scale <= 0)
    return -1;
  if (surface->width != width || surface->height != height ||
      surface->scale != scale) {
    surface->width = width;
    surface->height = height;
    surface->scale = scale;
    ++surface->generation;
    surface->notified = false;
  }
  notify(surface);
  return 1;
}
int32_t scoop_surface_gles_demand(SSSurface *surface, bool required,
                                  bool drawable) {
  surface->wanted = required && drawable;
  if (!surface->wanted) {
    surface->notified = false;
    if (surface->tick) {
      gtk_widget_remove_tick_callback(GTK_WIDGET(surface->area), surface->tick);
      surface->tick = 0;
    }
  } else {
    if (!surface->tick)
      surface->tick = gtk_widget_add_tick_callback(GTK_WIDGET(surface->area),
                                                   tick, surface, NULL);
    notify(surface);
  }
  return 1;
}
SSAcquired scoop_surface_gles_acquire(SSSurface *surface) {
  if (surface->acquired)
    return (SSAcquired){NULL, -1};
  surface->notified = false;
  int status = current(surface);
  if (status != 1)
    return (SSAcquired){NULL, status};
  if (!surface->ready || surface->presented)
    return (SSAcquired){NULL, 0};
  SSFrame *frame = malloc(sizeof(*frame));
  if (!frame)
    return (SSAcquired){NULL, -1};
  *frame = (SSFrame){surface, surface->generation};
  surface->acquired = true;
  surface->ready = false;
  return (SSAcquired){frame, 1};
}
void *scoop_surface_gles_frame_surface(SSFrame *frame) {
  return frame->surface->area;
}
int32_t scoop_surface_gles_frame_current(SSFrame *frame) {
  return frame->generation == frame->surface->generation
             ? current(frame->surface)
             : 0;
}
int32_t scoop_surface_gles_present(SSFrame *frame) {
  SSSurface *surface = frame->surface;
  int status = scoop_surface_gles_frame_current(frame);
  if (status != 1)
    return status;
  surface->presented = true;
  gtk_gl_area_queue_render(surface->area);
  return 1;
}
void scoop_surface_gles_frame_close(SSFrame *frame) {
  SSSurface *surface = frame->surface;
  surface->acquired = false;
  if (!surface->presented) {
    surface->ready = true;
    notify(surface);
  }
  free(frame);
}
int32_t scoop_surface_gles_close(SSSurface *surface) {
  surface->closed = true;
  surface->wanted = false;
  if (surface->tick)
    gtk_widget_remove_tick_callback(GTK_WIDGET(surface->area), surface->tick);
  g_signal_handlers_disconnect_by_data(surface->area, surface);
  gtk_widget_unparent(GTK_WIDGET(surface->area));
  g_object_unref(surface->area);
  g_object_unref(surface->gl);
  free(surface);
  return 1;
}
