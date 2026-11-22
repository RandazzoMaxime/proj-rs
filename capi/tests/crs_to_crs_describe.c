/* What the object of proj_create_crs_to_crs says of itself, before and after
   proj_normalize_for_visualization: PROJ returns the operation itself when one candidate is left
   (crs_to_crs.cpp:622-647, 775-780), with its ISO object, and a "Set of coordinate operations"
   without one otherwise. For each pair (source target ...): type, name, accuracy, the start of
   its WKT and PROJJSON, and proj_pj_info. run.sh compares the output with libproj 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <string.h>

static void describe(const char *label, PJ *op) {
    const char *name = proj_get_name(op);
    const char *wkt = proj_as_wkt(NULL, op, PJ_WKT2_2019, NULL);
    const char *json = proj_as_projjson(NULL, op, NULL);
    PJ_PROJ_INFO info = proj_pj_info(op);
    printf("  %s type=%d name=%s accuracy=%g\n", label, (int)proj_get_type(op), name ? name : "-",
           proj_coordoperation_get_accuracy(NULL, op));
    printf("    wkt=%.60s\n", wkt ? wkt : "-");
    printf("    json=%.40s\n", json ? json : "-");
    printf("    info: %s | %g\n", info.description ? info.description : "-", info.accuracy);
}

int main(int argc, char **argv) {
    for (int i = 1; i + 1 < argc; i += 2) {
        PJ *op = proj_create_crs_to_crs(NULL, argv[i], argv[i + 1], NULL);
        printf("%s %s\n", argv[i], argv[i + 1]);
        if (op == NULL) {
            printf("  null\n");
            continue;
        }
        describe("raw", op);
        PJ *normalized = proj_normalize_for_visualization(NULL, op);
        if (normalized) {
            describe("normalized", normalized);
            proj_destroy(normalized);
        }
        proj_destroy(op);
    }
    return 0;
}
