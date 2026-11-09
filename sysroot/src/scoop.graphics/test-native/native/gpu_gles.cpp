#include <EGL/egl.h>
#include <GLES3/gl3.h>
#include <atomic>
#include <cstdint>
#include <memory>

struct GpuFrame {
  std::atomic<int32_t> retains{0}, releases{0}, references{1};
  GLuint texture = 0;
  ~GpuFrame() { glDeleteTextures(1, &texture); }
};
static void drop(GpuFrame *frame) {
  if (--frame->references == 0)
    delete frame;
}
static void retain_frame(void *pointer) {
  auto *frame = static_cast<GpuFrame *>(pointer);
  ++frame->retains;
  ++frame->references;
}
static void release_frame(void *pointer, uint32_t) {
  auto *frame = static_cast<GpuFrame *>(pointer);
  ++frame->releases;
  drop(frame);
}
extern "C" {
GpuFrame *scoop_graphics_test_gpu_create(void *, void *, void *, int32_t width,
                                         int32_t height, uint64_t format) {
  try {
    auto frame = std::make_unique<GpuFrame>();
    glGenTextures(1, &frame->texture);
    glBindTexture(GL_TEXTURE_2D, frame->texture);
    glTexStorage2D(GL_TEXTURE_2D, 1, static_cast<GLenum>(format), width,
                   height);
    return glGetError() == GL_NO_ERROR ? frame.release() : nullptr;
  } catch (...) {
    return nullptr;
  }
}
void *scoop_graphics_test_gpu_texture(GpuFrame *) { return nullptr; }
uint32_t scoop_graphics_test_gpu_name(GpuFrame *frame) {
  return frame->texture;
}
int32_t scoop_graphics_test_gpu_metric(GpuFrame *frame, int32_t kind) {
  switch (kind) {
  case 0:
    return frame->retains;
  case 1:
    return frame->releases;
  case 2:
    return frame->references;
  default:
    return 0;
  }
}
void (*scoop_graphics_test_gpu_retain())(void *) { return retain_frame; }
void (*scoop_graphics_test_gpu_release())(void *, uint32_t) {
  return release_frame;
}
bool scoop_graphics_test_gpu_fill(GpuFrame *, void *, uint32_t texture,
                                  uint32_t, double r, double g, double b,
                                  double a) {
  GLuint fbo = 0;
  glGenFramebuffers(1, &fbo);
  glBindFramebuffer(GL_FRAMEBUFFER, fbo);
  glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D,
                         texture, 0);
  bool ok = glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE;
  if (ok) {
    glDisable(GL_SCISSOR_TEST);
    glColorMask(GL_TRUE, GL_TRUE, GL_TRUE, GL_TRUE);
    glClearColor(static_cast<float>(r), static_cast<float>(g),
                 static_cast<float>(b), static_cast<float>(a));
    glClear(GL_COLOR_BUFFER_BIT);
  }
  glDeleteFramebuffers(1, &fbo);
  return ok && glGetError() == GL_NO_ERROR;
}
bool scoop_graphics_test_gpu_close(GpuFrame *frame) {
  glFinish();
  drop(frame);
  return true;
}
}
