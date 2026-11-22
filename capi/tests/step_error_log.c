/* The line proj_trans logs at error level when fwd_prepare refuses a point (fwd.cpp:39-80):
   "<step>: Invalid latitude" or "<step>: Invalid longitude", for a single operation, each step of
   a pipeline that reads radians, and each candidate of a CRS pair that is tried. run.sh compares
   the log lines, the coordinates and the error codes with libproj of PROJ 9.9.0. */
#include <math.h>
#include <proj.h>
#include <stdio.h>

static void logger(void *data, int level, const char *message) {
    (void)data;
    printf("  log %d: %s\n", level, message);
}

static void run(PJ_CONTEXT *ctx, PJ *op, const char *label, PJ_DIRECTION way, double x, double y) {
    printf("%s %s (%g, %g)\n", label, way == PJ_FWD ? "fwd" : "inv", x, y);
    if (!op) {
        puts("  null");
        return;
    }
    proj_errno_reset(op);
    PJ_COORD out = proj_trans(op, way, proj_coord(x, y, 0, HUGE_VAL));
    printf("  out %g %g errno=%d\n", out.xy.x, out.xy.y, proj_context_errno(ctx));
}

int main(void) {
    PJ_CONTEXT *ctx = proj_context_create();
    proj_log_func(ctx, NULL, logger);
    const char *strings[] = {
        "+proj=tmerc +ellps=airy",
        "+proj=cart +ellps=WGS84",
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=tmerc "
        "+lat_0=49 +lon_0=-2 +k=0.9996012717 +x_0=400000 +y_0=-100000 +ellps=airy",
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=push +v_3 "
        "+step +proj=cart +ellps=WGS84 +step +inv +proj=cart +ellps=airy +step +proj=pop +v_3 "
        "+step +proj=tmerc +ellps=airy",
    };
    for (int i = 0; i < 4; i++) {
        PJ *op = proj_create(ctx, strings[i]);
        run(ctx, op, strings[i], PJ_FWD, 100000, 100000);
        run(ctx, op, strings[i], PJ_FWD, 20, 0.5);
        run(ctx, op, strings[i], PJ_INV, 1e30, 1e30);
        proj_destroy(op);
    }
    const char *pairs[][2] = {{"EPSG:4326", "EPSG:27700"}, {"EPSG:4326", "EPSG:2154"}};
    for (int i = 0; i < 2; i++) {
        PJ *op = proj_create_crs_to_crs(ctx, pairs[i][0], pairs[i][1], NULL);
        PJ *normalized = op ? proj_normalize_for_visualization(ctx, op) : NULL;
        run(ctx, normalized, pairs[i][1], PJ_FWD, 100000, 100000);
        run(ctx, normalized, pairs[i][1], PJ_FWD, 2, 49);
        proj_destroy(normalized);
        proj_destroy(op);
    }
    proj_context_destroy(ctx);
    return 0;
}
