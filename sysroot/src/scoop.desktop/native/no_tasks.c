#include "desktop.h"

SDResult scoop_desktop_tasks_create(void *application) {
  return (SDResult){NULL, SD_UNSUPPORTED};
}
int64_t scoop_desktop_tasks_add(void *tasks, const uint8_t *title,
                                int64_t title_length,
                                const uint8_t *description,
                                int64_t description_length,
                                const uint16_t *icon, int64_t icon_length,
                                int32_t index) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_tasks_argument(void *tasks, const uint8_t *text,
                                     int64_t length) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_tasks_commit(void *tasks) { return SD_UNSUPPORTED; }
void scoop_desktop_tasks_destroy(void *tasks) { (void)tasks; }
