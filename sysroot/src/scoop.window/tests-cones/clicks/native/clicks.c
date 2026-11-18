#include "../../../native/windows/input/clicks.h"

#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      return __LINE__;                                                         \
  } while (0)

static int press(SWClicks *clicks, uint32_t time, int button, double x,
                 double y, bool twice) {
  return sw_clicks_press(clicks, time, button, x, y, 500, 4, 6, twice);
}

int32_t scoop_test_click_sequence(int32_t scenario) {
  SWClicks clicks = {0};
  switch (scenario) {
  case 0:
    CHECK(press(&clicks, 100, 0, 20, 30, false) == 1);
    CHECK(press(&clicks, 120, 0, 20, 30, true) == 2);
    CHECK(press(&clicks, 140, 0, 20, 30, false) == 3);
    CHECK(press(&clicks, 160, 0, 20, 30, true) == 4);
    clicks = (SWClicks){0};
    CHECK(press(&clicks, 200, 0, 20, 30, true) == 2);
    CHECK(press(&clicks, 220, 0, 20, 30, false) == 3);
    break;
  case 1:
    CHECK(press(&clicks, 100, 0, 0, 0, false) == 1);
    CHECK(press(&clicks, 600, 0, 0, 0, false) == 2);
    CHECK(press(&clicks, 1101, 0, 0, 0, false) == 1);
    clicks = (SWClicks){0};
    CHECK(press(&clicks, UINT32_MAX - 15, 0, 0, 0, false) == 1);
    CHECK(press(&clicks, 32, 0, 0, 0, false) == 2);
    CHECK(press(&clicks, 31, 0, 0, 0, false) == 1);
    break;
  case 2:
    CHECK(press(&clicks, 100, 0, 20, 30, false) == 1);
    CHECK(press(&clicks, 120, 0, 18, 27, false) == 2);
    CHECK(press(&clicks, 140, 0, 20, 27, false) == 1);
    CHECK(press(&clicks, 160, 0, 20, 30, false) == 1);
    CHECK(press(&clicks, 180, 0, 40, 60, false) == 1);
    CHECK(press(&clicks, 200, 0, 40, 60, true) == 2);
    break;
  case 3:
    CHECK(press(&clicks, 100, 0, 20, 30, false) == 1);
    sw_clicks_move(&clicks, 24, 33, 4, 3);
    CHECK(clicks.count == 1);
    sw_clicks_move(&clicks, 24.5, 30, 4, 3);
    CHECK(clicks.count == 0);
    sw_clicks_move(&clicks, 20, 30, 4, 3);
    CHECK(press(&clicks, 120, 0, 20, 30, true) == 1);
    CHECK(press(&clicks, 140, 0, 20, 30, false) == 2);
    sw_clicks_move(&clicks, 20, 33.5, 4, 3);
    CHECK(press(&clicks, 160, 0, 20, 30, false) == 1);
    break;
  case 4:
    CHECK(press(&clicks, 100, 0, 20, 30, false) == 1);
    CHECK(press(&clicks, 120, 0, 20, 30, true) == 2);
    sw_clicks_reset(&clicks);
    CHECK(press(&clicks, 140, 0, 20, 30, true) == 1);
    CHECK(press(&clicks, 160, 1, 20, 30, false) == 1);
    CHECK(press(&clicks, 180, 0, 20, 30, false) == 1);
    clicks.count = INT_MAX;
    CHECK(press(&clicks, 200, 0, 20, 30, false) == INT_MAX);
    break;
  case 5:
    CHECK(sw_clicks_press(&clicks, 100, 0, 20, 30, 100, 4, 6, false) == 1);
    CHECK(sw_clicks_press(&clicks, 400, 0, 27, 30, 500, 16, 6, false) == 2);
    CHECK(sw_clicks_press(&clicks, 450, 0, 30, 30, 100, 4, 6, false) == 1);
    CHECK(sw_clicks_press(&clicks, 551, 0, 30, 30, 100, 4, 6, false) == 1);
    break;
  default:
    return -1;
  }
  return 0;
}
