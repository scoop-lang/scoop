#include "../shared/graphics.h"
#include "include/gpu/ganesh/GrBackendSurface.h"
#include "include/gpu/ganesh/SkSurfaceGanesh.h"
#include "include/gpu/ganesh/gl/GrGLAssembleInterface.h"
#include "include/gpu/ganesh/gl/GrGLBackendSurface.h"
#include "include/gpu/ganesh/gl/GrGLDirectContext.h"
#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_14
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <cstring>
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include <map>
#include <mutex>

struct SGGLDisplay {
  EGLDisplay display;
  explicit SGGLDisplay(EGLDisplay value) : display(value) {}
  ~SGGLDisplay() { eglTerminate(display); }
};

static std::shared_ptr<SGGLDisplay> initialize_display(EGLDisplay display) {
  static std::mutex mutex;
  static std::map<EGLDisplay, std::weak_ptr<SGGLDisplay>> displays;
  std::lock_guard lock(mutex);
  auto existing = displays[display].lock();
  if (existing)
    return existing;
  EGLint major, minor;
  if (!eglInitialize(display, &major, &minor))
    return {};
  auto owner = std::make_shared<SGGLDisplay>(display);
  if (major < 1 || (major == 1 && minor < 5))
    return {};
  displays[display] = owner;
  return owner;
}

struct SGGLState {
  EGLDisplay display = EGL_NO_DISPLAY;
  EGLConfig config = nullptr;
  EGLContext native = EGL_NO_CONTEXT;
  sk_sp<GrDirectContext> context;
  GdkGLContext *gtk = nullptr;
  std::shared_ptr<SGGLDisplay> displayOwner;

  bool activate(EGLSurface surface = EGL_NO_SURFACE) {
    if (native == EGL_NO_CONTEXT || (context && context->abandoned()))
      return false;
    if (gtk) {
      bool changed = gdk_gl_context_get_current() != gtk ||
                     eglGetCurrentContext() != native;
      if (changed && gdk_gl_context_get_current() == gtk)
        gdk_gl_context_clear_current();
      gdk_gl_context_make_current(gtk);
      if (changed && context)
        context->resetContext();
      return eglGetCurrentContext() == native;
    }
    if (eglGetCurrentContext() == native &&
        (surface == EGL_NO_SURFACE ||
         eglGetCurrentSurface(EGL_DRAW) == surface))
      return true;
    if (gdk_gl_context_get_current())
      gdk_gl_context_clear_current();
    if (eglMakeCurrent(display, surface, surface, native))
      return true;
    if (context)
      context->abandonContext();
    return false;
  }
  bool close() {
    if (native == EGL_NO_CONTEXT)
      return true;
    bool ok = activate();
    if (context) {
      if (ok) {
        ok = context->flushAndSubmit(GrSyncCpu::kYes).fSuccess;
        context->releaseResourcesAndAbandonContext();
      } else
        context->abandonContext();
      context.reset();
    }
    if (gtk) {
      if (gdk_gl_context_get_current() == gtk)
        gdk_gl_context_clear_current();
      g_clear_object(&gtk);
      native = EGL_NO_CONTEXT;
      display = EGL_NO_DISPLAY;
      return ok;
    }
    if (eglGetCurrentContext() == native)
      ok = eglMakeCurrent(display, EGL_NO_SURFACE, EGL_NO_SURFACE,
                          EGL_NO_CONTEXT) &&
           ok;
    ok = eglDestroyContext(display, native) && ok;
    native = EGL_NO_CONTEXT;
    displayOwner.reset();
    display = EGL_NO_DISPLAY;
    return ok;
  }
  ~SGGLState() { close(); }
};

extern "C" {
bool scoop_graphics_gles_activate(SGObject *object) {
  return !object->gl || object->gl->activate(object->glSurface);
}

SGContext *scoop_graphics_gles_context(int32_t platform, void *connection,
                                       uint32_t visual) {
  try {
    auto gl = std::make_shared<SGGLState>();
    EGLenum kind = platform == 1   ? EGL_PLATFORM_XCB_EXT
                   : platform == 2 ? EGL_PLATFORM_WAYLAND_KHR
                                   : EGL_PLATFORM_SURFACELESS_MESA;
    gl->display = eglGetPlatformDisplay(kind, connection, nullptr);
    if (gl->display == EGL_NO_DISPLAY)
      return nullptr;
    gl->displayOwner = initialize_display(gl->display);
    if (!gl->displayOwner || !eglBindAPI(EGL_OPENGL_ES_API))
      return nullptr;
    const char *extensions = eglQueryString(gl->display, EGL_EXTENSIONS);
    if (!extensions || !strstr(extensions, "EGL_KHR_surfaceless_context"))
      return nullptr;
    EGLint attributes[] = {EGL_RENDERABLE_TYPE,
                           EGL_OPENGL_ES3_BIT,
                           EGL_SURFACE_TYPE,
                           platform ? EGL_WINDOW_BIT : EGL_PBUFFER_BIT,
                           EGL_RED_SIZE,
                           8,
                           EGL_GREEN_SIZE,
                           8,
                           EGL_BLUE_SIZE,
                           8,
                           EGL_ALPHA_SIZE,
                           8,
                           EGL_STENCIL_SIZE,
                           8,
                           EGL_NONE};
    EGLint count = 0;
    if (!eglChooseConfig(gl->display, attributes, nullptr, 0, &count) ||
        count <= 0)
      return nullptr;
    std::vector<EGLConfig> configs(count);
    if (!eglChooseConfig(gl->display, attributes, configs.data(), count,
                         &count))
      return nullptr;
    for (EGLint index = 0; index < count; ++index) {
      EGLint nativeVisual = 0;
      if (platform == 1 &&
          (!eglGetConfigAttrib(gl->display, configs[index],
                               EGL_NATIVE_VISUAL_ID, &nativeVisual) ||
           static_cast<uint32_t>(nativeVisual) != visual))
        continue;
      gl->config = configs[index];
      break;
    }
    if (!gl->config)
      return nullptr;
    const EGLint contextAttributes[] = {EGL_CONTEXT_CLIENT_VERSION, 3,
                                        EGL_NONE};
    gl->native = eglCreateContext(gl->display, gl->config, EGL_NO_CONTEXT,
                                  contextAttributes);
    if (!gl->activate())
      return nullptr;
    auto interface =
        GrGLMakeAssembledGLESInterface(nullptr, [](void *, const char *name) {
          return eglGetProcAddress(name);
        });
    gl->context = GrDirectContexts::MakeGL(std::move(interface));
    if (!gl->context)
      return nullptr;
    auto result = std::make_unique<SGContext>();
    result->context = gl->context;
    result->gl = std::move(gl);
    return result.release();
  } catch (...) {
    return nullptr;
  }
}

SGContext *scoop_graphics_gles_gtk_context(void *display) {
  try {
    auto gl = std::make_shared<SGGLState>();
    GError *error = nullptr;
    gl->gtk = gdk_display_create_gl_context(GDK_DISPLAY(display), &error);
    if (!gl->gtk) {
      g_clear_error(&error);
      return nullptr;
    }
    gdk_gl_context_set_allowed_apis(gl->gtk, GDK_GL_API_GLES);
    gdk_gl_context_set_required_version(gl->gtk, 3, 0);
    if (!gdk_gl_context_realize(gl->gtk, &error)) {
      g_clear_error(&error);
      g_clear_object(&gl->gtk);
      return nullptr;
    }
    gdk_gl_context_make_current(gl->gtk);
    gl->display = eglGetCurrentDisplay();
    gl->native = eglGetCurrentContext();
    if (gl->native == EGL_NO_CONTEXT) {
      g_clear_object(&gl->gtk);
      return nullptr;
    }
    auto interface =
        GrGLMakeAssembledGLESInterface(nullptr, [](void *, const char *name) {
          return eglGetProcAddress(name);
        });
    gl->context = GrDirectContexts::MakeGL(std::move(interface));
    if (!gl->context)
      return nullptr;
    auto result = std::make_unique<SGContext>();
    result->context = gl->context;
    result->gl = std::move(gl);
    return result.release();
  } catch (...) {
    return nullptr;
  }
}
void *scoop_graphics_gles_gtk_native(SGContext *context) {
  return context->gl->gtk;
}
SGTarget *scoop_graphics_gles_gtk_target(SGContext *context, void *view,
                                         int32_t width, int32_t height) {
  try {
    GtkGLArea *area = GTK_GL_AREA(view);
    if (!context->gl->gtk ||
        gtk_gl_area_get_context(area) != context->gl->gtk ||
        !context->gl->activate())
      return nullptr;
    gtk_gl_area_attach_buffers(area);
    if (gtk_gl_area_get_error(area))
      return nullptr;
    GLint framebuffer, stencil, samples;
    glGetIntegerv(GL_DRAW_FRAMEBUFFER_BINDING, &framebuffer);
    glGetIntegerv(GL_STENCIL_BITS, &stencil);
    glGetIntegerv(GL_SAMPLES, &samples);
    context->context->resetContext();
    GrGLFramebufferInfo info = {static_cast<GrGLuint>(framebuffer), GL_RGBA8};
    auto backend =
        GrBackendRenderTargets::MakeGL(width, height, samples, stencil, info);
    auto result = std::make_unique<SGTarget>();
    result->context = context->context;
    result->gl = context->gl;
    result->surface = SkSurfaces::WrapBackendRenderTarget(
        context->context.get(), backend, kBottomLeft_GrSurfaceOrigin,
        kRGBA_8888_SkColorType, SkColorSpace::MakeSRGB(), nullptr);
    return result->surface ? result.release() : nullptr;
  } catch (...) {
    return nullptr;
  }
}

void *scoop_graphics_gles_display(SGContext *context) {
  return context->gl->display;
}
void *scoop_graphics_gles_config(SGContext *context) {
  return context->gl->config;
}
void *scoop_graphics_gles_native(SGContext *context) {
  return context->gl->native;
}

SGTarget *scoop_graphics_gles_target(SGContext *context, void *surface,
                                     int32_t width, int32_t height) {
  try {
    auto result = std::make_unique<SGTarget>();
    result->context = context->context;
    result->gl = context->gl;
    result->glSurface = surface;
    if (!result->gl->activate(surface))
      return nullptr;
    glBindFramebuffer(GL_FRAMEBUFFER, 0);
    GLint stencil = 0, samples = 0;
    glGetIntegerv(GL_STENCIL_BITS, &stencil);
    glGetIntegerv(GL_SAMPLES, &samples);
    context->context->resetContext();
    GrGLFramebufferInfo info = {0, GL_RGBA8};
    auto backend =
        GrBackendRenderTargets::MakeGL(width, height, samples, stencil, info);
    result->surface = SkSurfaces::WrapBackendRenderTarget(
        context->context.get(), backend, kBottomLeft_GrSurfaceOrigin,
        kRGBA_8888_SkColorType, SkColorSpace::MakeSRGB(), nullptr);
    return result->surface ? result.release() : nullptr;
  } catch (...) {
    return nullptr;
  }
}

SGTarget *scoop_graphics_gles_offscreen(SGContext *context, int32_t width,
                                        int32_t height) {
  try {
    auto result = std::make_unique<SGTarget>();
    result->context = context->context;
    result->gl = context->gl;
    result->external = false;
    if (!result->gl->activate())
      return nullptr;
    result->surface = SkSurfaces::RenderTarget(
        context->context.get(), skgpu::Budgeted::kYes, sg_info(width, height));
    return result->surface ? result.release() : nullptr;
  } catch (...) {
    return nullptr;
  }
}

int32_t scoop_graphics_gles_drain(SGContext *context) {
  try {
    return context->gl->activate() &&
           context->context->flushAndSubmit(GrSyncCpu::kYes).fSuccess;
  } catch (...) {
    return 0;
  }
}
int32_t scoop_graphics_gles_close(SGContext *context) {
  try {
    return context->gl->close();
  } catch (...) {
    return 0;
  }
}
}
