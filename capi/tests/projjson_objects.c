/* proj_create on the PROJJSON of objects that are not CRS (ellipsoid, prime meridian, datum,
   datum ensemble, coordinate system, operations), as pyproj builds them. Each object comes from
   the database, goes out with proj_as_projjson and back in with proj_create. Run against two
   libraries, the outputs must be identical. */
#include <stdio.h>

#include "proj.h"

static void report(PJ_CONTEXT *ctx, const char *label, PJ *object) {
    if (!object) {
        printf("%s | missing\n", label);
        return;
    }
    const char *json = proj_as_projjson(ctx, object, NULL);
    PJ *back = json ? proj_create(ctx, json) : NULL;
    printf("%s | type %d | back %d | name %s | equivalent %d\n", label, (int)proj_get_type(object),
           back ? (int)proj_get_type(back) : -1, back ? proj_get_name(back) : "-",
           back ? proj_is_equivalent_to(object, back, PJ_COMP_EQUIVALENT) : -1);
    proj_destroy(back);
    proj_destroy(object);
}

int main(void) {
    PJ_CONTEXT *ctx = proj_context_create();
    struct {
        const char *code;
        PJ_CATEGORY category;
    } items[] = {{"7030", PJ_CATEGORY_ELLIPSOID},
                 {"8903", PJ_CATEGORY_PRIME_MERIDIAN},
                 {"6258", PJ_CATEGORY_DATUM},
                 {"6326", PJ_CATEGORY_DATUM_ENSEMBLE},
                 {"5101", PJ_CATEGORY_DATUM},
                 {"1671", PJ_CATEGORY_COORDINATE_OPERATION},
                 {"16031", PJ_CATEGORY_COORDINATE_OPERATION},
                 {"8047", PJ_CATEGORY_COORDINATE_OPERATION},
                 {NULL, PJ_CATEGORY_ELLIPSOID}};
    for (int i = 0; items[i].code; i++) {
        char label[32];
        snprintf(label, sizeof label, "EPSG:%s", items[i].code);
        report(ctx, label,
               proj_create_from_database(ctx, "EPSG", items[i].code, items[i].category, 0, NULL));
    }
    const char *crs_codes[] = {"EPSG:4326", "EPSG:32631", "EPSG:4978", "EPSG:5703", NULL};
    for (int i = 0; crs_codes[i]; i++) {
        PJ *crs = proj_create(ctx, crs_codes[i]);
        char label[48];
        snprintf(label, sizeof label, "cs of %s", crs_codes[i]);
        report(ctx, label, crs ? proj_crs_get_coordinate_system(ctx, crs) : NULL);
        proj_destroy(crs);
    }
    proj_context_destroy(ctx);
    return 0;
}
