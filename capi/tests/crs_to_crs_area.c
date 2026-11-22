/* proj_create_crs_to_crs_from_pj with an area of interest: the factory keeps the operations whose
   extent meets the box (crs_to_crs.cpp:557-561), so the pair has fewer candidates, here one.
   run.sh compares the description and the operation count with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <stdlib.h>

static void show(PJ_CONTEXT *ctx, const char *source, const char *target, const double *box) {
    PJ *from = proj_create(ctx, source), *to = proj_create(ctx, target);
    PJ_AREA *area = NULL;
    if (box) {
        area = proj_area_create();
        proj_area_set_bbox(area, box[0], box[1], box[2], box[3]);
    }
    PJ *op = proj_create_crs_to_crs_from_pj(ctx, from, to, area, NULL);
    PJ_PROJ_INFO info = op ? proj_pj_info(op) : (PJ_PROJ_INFO){0};
    printf("%s -> %s area=%s: %s\n", source, target, box ? "yes" : "no",
           op ? info.description : "null");
    proj_destroy(op);
    proj_area_destroy(area);
    proj_destroy(from);
    proj_destroy(to);
}

int main(void) {
    PJ_CONTEXT *ctx = proj_context_create();
    const double chatham[4] = {-177.25, -44.64, -175.54, -43.3};
    const double paris[4] = {2.2, 48.8, 2.5, 48.9};
    show(ctx, "EPSG:7789", "EPSG:4936", NULL);
    show(ctx, "EPSG:7789", "EPSG:4936", chatham);
    show(ctx, "EPSG:4326", "EPSG:2154", NULL);
    show(ctx, "EPSG:4326", "EPSG:2154", paris);
    show(ctx, "EPSG:4267", "EPSG:4269", paris);
    proj_context_destroy(ctx);
    return 0;
}
