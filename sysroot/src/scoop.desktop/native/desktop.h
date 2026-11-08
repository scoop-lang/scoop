#ifndef SCOOP_DESKTOP_H
#define SCOOP_DESKTOP_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
  SD_UNSUPPORTED = -1,
  SD_UNAVAILABLE = -2,
  SD_INVALID = -4,
  SD_BUSY = -5,
  SD_REQUESTED = -6,
  SD_FAILURE = -7
};
typedef struct {
  void *pointer;
  int64_t error;
} SDResult;
typedef int32_t (*SDCallback)(void *, int32_t);
typedef struct SDPixels {
  int32_t width, height;
  struct SDPixels *next;
  uint8_t bytes[];
} SDPixels;
typedef struct {
  double width, height;
  int32_t is_template;
  size_t references;
  SDPixels *pixels, *last;
} SDImage;

SDImage *sd_retain_image(SDImage *image);
void scoop_desktop_image_destroy(SDImage *image);
SDResult scoop_desktop_image_create(double width, double height,
                                    int32_t is_template);
int64_t scoop_desktop_image_add(SDImage *, int32_t, int32_t, const uint8_t *);
SDResult scoop_desktop_application_create(SDCallback, void *, const uint8_t *,
                                          int64_t);
int32_t scoop_desktop_application_capabilities(void *);
int32_t scoop_desktop_application_busy(void *);
void scoop_desktop_application_cancel(void *);
void scoop_desktop_application_destroy(void *);
int64_t scoop_desktop_application_icon(void *, SDImage *);
int64_t scoop_desktop_application_badge(void *, const uint8_t *, int64_t);
int64_t scoop_desktop_application_menu(void *, void *);
SDResult scoop_desktop_window_create(void *, void *);
int32_t scoop_desktop_window_capabilities(void *);
void scoop_desktop_window_destroy(void *);
int64_t scoop_desktop_window_icon(void *, SDImage *);
int64_t scoop_desktop_window_overlay(void *, SDImage *, const uint8_t *,
                                     int64_t);
int64_t scoop_desktop_window_progress(void *, int32_t, uint64_t, uint64_t);
int64_t scoop_desktop_window_caption(void *, int32_t, const uint8_t *, int64_t);
SDResult scoop_desktop_tasks_create(void *);
int64_t scoop_desktop_tasks_add(void *, const uint8_t *, int64_t,
                                const uint8_t *, int64_t, const uint16_t *,
                                int64_t, int32_t);
int64_t scoop_desktop_tasks_argument(void *, const uint8_t *, int64_t);
int64_t scoop_desktop_tasks_commit(void *);
void scoop_desktop_tasks_destroy(void *);

#ifdef __cplusplus
}
#endif
#endif
