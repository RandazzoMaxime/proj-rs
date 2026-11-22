/* proj_create refuses a PROJ string whose common parameters pj_init rejects (init.cpp:653-678):
   |lat_0| beyond 90 degrees, or a scale factor not above 0, for a CRS (+type=crs) and for an
   operation. run.sh compares the result, the error code and the logged line with libproj 9.9.0. */
#include <proj.h>
#include <stdio.h>

static void log_error(void *data, int level, const char *message) {
    (void)data;
    if (level == PJ_LOG_ERROR) printf("  log: %s\n", message);
}

int main(void) {
    const char *texts[] = {
        "+proj=tmerc +k=0 +type=crs", "+proj=tmerc +k_0=-2 +type=crs",
        "+proj=utm +zone=31 +k=0 +type=crs", "+proj=longlat +k=0 +type=crs",
        "+proj=tmerc +lat_0=100 +type=crs", "+proj=tmerc +k=0", "+proj=merc +lat_0=-91",
        "+proj=tmerc +k=0.9996 +type=crs", "+proj=tmerc +lat_0=90 +type=crs",
    };
    for (size_t i = 0; i < sizeof texts / sizeof texts[0]; i++) {
        PJ_CONTEXT *ctx = proj_context_create();
        proj_log_func(ctx, NULL, log_error);
        PJ *object = proj_create(ctx, texts[i]);
        printf("%s -> %s, errno %d\n", texts[i], object ? "object" : "null", proj_context_errno(ctx));
        proj_destroy(object);
        proj_context_destroy(ctx);
    }
    return 0;
}
