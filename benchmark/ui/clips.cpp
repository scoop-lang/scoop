#include "graphics.h"
#include <chrono>
#include <cstdio>

extern "C" {
int32_t scoop_graphics_canvas_save(SGCanvas *, double, bool, SGFilter *,
                                   int32_t, bool, SGRect);
int32_t scoop_graphics_canvas_clip_rect(SGCanvas *, SGRect);
int32_t scoop_graphics_canvas_clip_path(SGCanvas *, SGPath *);
SGGeometry *scoop_graphics_geometry_rect(SGCanvas *, SGRect);
int32_t scoop_graphics_geometry_hit(SGGeometry *, SGPoint);
}

int main() {
  for (bool curves : {false, true}) {
    for (int depth : {8, 64, 256, 1024}) {
      const int iterations = 20000 / depth;
      auto surface = SkSurfaces::Raster(sg_info(2048, 2048));
      auto start = std::chrono::steady_clock::now();
      int hits = 0;
      for (int round = 0; round < iterations; ++round) {
        auto recording = std::make_shared<SGRecording>();
        auto bounds = SkRect::MakeWH(2048, 2048);
        recording->recorder.beginRecording(bounds);
        SGCanvas canvas(surface, nullptr, recording, bounds);
        std::vector<std::unique_ptr<SGGeometry>> snapshots;
        for (int index = 0; index < depth; ++index) {
          scoop_graphics_canvas_save(&canvas, 1, false, nullptr, 0, false, {});
          const SGRect rect = {double(index) / 4, double(index) / 4,
                               2048.0 - index / 2.0, 2048.0 - index / 2.0};
          if (curves) {
            SGPath path(SkPathBuilder().addOval(sg_rect(rect)).detach());
            scoop_graphics_canvas_clip_path(&canvas, &path);
          } else {
            scoop_graphics_canvas_clip_rect(&canvas, rect);
          }
          snapshots.emplace_back(
              scoop_graphics_geometry_rect(&canvas, {0, 0, 2048, 2048}));
        }
        hits +=
            scoop_graphics_geometry_hit(snapshots.back().get(), {1024, 1024});
      }
      auto elapsed = std::chrono::duration<double, std::micro>(
                         std::chrono::steady_clock::now() - start)
                         .count();
      std::printf("%s depth=%d us/pass=%.2f hits=%d\n",
                  curves ? "paths" : "rectangles", depth, elapsed / iterations,
                  hits);
      if (hits != iterations)
        return 1;
    }
  }
}
