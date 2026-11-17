/* The same program as bench/footprint/rs/src/main.rs, through libproj's C API.
   Stdout is `ok=<n> fail=<n> sum=<x>`, stderr `vmhwm_kb=<n>`. Build against
   PROJ 9.9.0 or against the libproj of proj-rs's capi/. One mode per process. */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "proj.h"

#define N 1000000u

static uint64_t state = 6;

static double unit(void) {
  state = state * 6364136223846793005ULL + 1;
  return (double)(state >> 11) / 9007199254740992.0;
}

static void next_point(double *lon, double *lat) {
  *lon = -4.5 + unit() * 12.5;
  *lat = 41.5 + unit() * 9.5;
}

static void report_peak_rss(void) {
  char line[256];
  FILE *status = fopen("/proc/self/status", "r");
  while (status && fgets(line, sizeof line, status))
    if (!strncmp(line, "VmHWM:", 6)) fprintf(stderr, "vmhwm_kb=%ld\n", atol(line + 6));
  if (status) fclose(status);
}

static PJ *create(PJ_CONTEXT *ctx, const char *from, const char *to) {
  PJ *raw = proj_create_crs_to_crs(ctx, from, to, NULL);
  if (!raw) {
    fprintf(stderr, "proj_create_crs_to_crs %s %s failed\n", from, to);
    exit(1);
  }
  PJ *pj = proj_normalize_for_visualization(ctx, raw);
  proj_destroy(raw);
  if (!pj) {
    fprintf(stderr, "proj_normalize_for_visualization failed\n");
    exit(1);
  }
  return pj;
}

static uint64_t ok, fail;
static double sum;

static void add(double x, double y) {
  if (isfinite(x) && isfinite(y)) {
    ok++;
    sum += x + y;
  } else {
    fail++;
  }
}

static void one_by_one(PJ *pj) {
  for (uint32_t i = 0; i < N; i++) {
    PJ_COORD c = {{0, 0, 0, HUGE_VAL}};
    next_point(&c.xyzt.x, &c.xyzt.y);
    c = proj_trans(pj, PJ_FWD, c);
    add(c.xy.x, c.xy.y);
  }
}

static void array(PJ *pj) {
  double *buffer = malloc((size_t)N * 2 * sizeof(double));
  for (uint32_t i = 0; i < N; i++) next_point(&buffer[2 * i], &buffer[2 * i + 1]);
  size_t done = proj_trans_generic(pj, PJ_FWD, buffer, 16, N, buffer + 1, 16, N, NULL, 0, 0, NULL, 0, 0);
  if (done != N) fail = N;
  else for (uint32_t i = 0; i < N; i++) add(buffer[2 * i], buffer[2 * i + 1]);
  free(buffer);
}

int main(int argc, char **argv) {
  const char *mode = argc > 1 ? argv[1] : "convert";
  if (!strcmp(mode, "empty")) {
    report_peak_rss();
    printf("ok=0 fail=0 sum=0.000000\n");
    return 0;
  }
  PJ_CONTEXT *ctx = proj_context_create();
  proj_log_level(ctx, PJ_LOG_NONE);
  if (!strcmp(mode, "multi")) {
    PJ *pj = create(ctx, "EPSG:4267", "EPSG:4326");
    one_by_one(pj);
    proj_destroy(pj);
  } else {
    PJ *pj = create(ctx, "EPSG:4326", "EPSG:2154");
    if (!strcmp(mode, "convert")) {
      PJ_COORD c = {{2.35, 48.85, 0, HUGE_VAL}};
      c = proj_trans(pj, PJ_FWD, c);
      add(c.xy.x, c.xy.y);
    } else if (!strcmp(mode, "points")) {
      one_by_one(pj);
    } else if (!strcmp(mode, "array")) {
      array(pj);
    } else if (strcmp(mode, "create")) {
      fprintf(stderr, "unknown mode %s\n", mode);
      return 2;
    }
    proj_destroy(pj);
  }
  proj_context_destroy(ctx);
  report_peak_rss();
  printf("ok=%llu fail=%llu sum=%.6f\n", (unsigned long long)ok, (unsigned long long)fail, sum);
  return 0;
}
