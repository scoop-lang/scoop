#include "desktop.h"
#include <errno.h>
#include <stdlib.h>
#include <string.h>

SDResult scoop_desktop_image_create(double width, double height,
                                    int32_t is_template) {
  SDImage *image = calloc(1, sizeof(*image));
  if (!image)
    return (SDResult){NULL, ENOMEM};
  image->width = width;
  image->height = height;
  image->is_template = is_template;
  image->references = 1;
  return (SDResult){image, 0};
}

int64_t scoop_desktop_image_add(SDImage *image, int32_t width, int32_t height,
                                const uint8_t *bytes) {
  if (width <= 0 || height <= 0 ||
      (size_t)width > (SIZE_MAX - sizeof(SDPixels)) / 4 / (size_t)height)
    return EINVAL;
  size_t length = (size_t)width * (size_t)height * 4;
  SDPixels *pixels = malloc(sizeof(*pixels) + length);
  if (!pixels)
    return ENOMEM;
  pixels->width = width;
  pixels->height = height;
  pixels->next = NULL;
  memcpy(pixels->bytes, bytes, length);
  if (image->last)
    image->last->next = pixels;
  else
    image->pixels = pixels;
  image->last = pixels;
  return 0;
}

SDImage *sd_retain_image(SDImage *image) {
  if (image)
    ++image->references;
  return image;
}
void scoop_desktop_image_destroy(SDImage *image) {
  if (!image || --image->references)
    return;
  while (image->pixels) {
    SDPixels *pixels = image->pixels;
    image->pixels = pixels->next;
    free(pixels);
  }
  free(image);
}
