/* proj_crs_get_coordinate_system takes a SingleCRS only (c_api.cpp:9044-9057): a bound or a
   compound CRS gives NULL and an error log. run.sh compares the output with
   libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>

static void logger(void *data, int level, const char *message) {
    (void)data;
    printf("  log %d: %s\n", level, message);
}

int main(int argc, char **argv) {
    PJ_CONTEXT *ctx = proj_context_create();
    proj_log_func(ctx, NULL, logger);
    for (int i = 1; i < argc; i++) {
        PJ *crs = proj_create(ctx, argv[i]);
        printf("%s: type=%d\n", argv[i], crs ? (int)proj_get_type(crs) : -1);
        if (!crs) continue;
        PJ *cs = proj_crs_get_coordinate_system(ctx, crs);
        printf("  cs=%s type=%d axes=%d\n", cs ? "yes" : "null",
               cs ? (int)proj_cs_get_type(ctx, cs) : -1, cs ? proj_cs_get_axis_count(ctx, cs) : -1);
        proj_destroy(cs);
        proj_destroy(crs);
    }
    proj_context_destroy(ctx);
    return 0;
}
