/* Operations built with NULL names, as GDAL's SetProjCS builds a placeholder conversion: PROJ's
   setSingleOperationElements names a missing operation, method, parameter or unit "unnamed" (an
   angular unit degree, a linear unit metre). Run against two libraries, the outputs must be
   identical. */
#include <stdio.h>

#include "proj.h"
/* GDAL's SetProjCS: a conversion with every argument NULL, then a projected CRS on it. */
int main(void) {
    PJ_CONTEXT *ctx = proj_context_create();
    PJ *conv = proj_create_conversion(ctx, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL);
    const char *method = NULL, *auth = NULL, *code = NULL;
    if (conv) proj_coordoperation_get_method_info(ctx, conv, &method, &auth, &code);
    printf("conversion %d | name %s | method %s\n", conv != NULL, conv ? proj_get_name(conv) : "-",
           method ? method : "-");
    PJ *geog = proj_create(ctx, "EPSG:4326");
    PJ *cs = proj_create_cartesian_2D_cs(ctx, PJ_CART2D_EASTING_NORTHING, NULL, 0);
    PJ *crs = conv ? proj_create_projected_crs(ctx, "my crs", geog, conv, cs) : NULL;
    printf("projected %d | type %d | name %s\n", crs != NULL, crs ? (int)proj_get_type(crs) : -1,
           crs ? proj_get_name(crs) : "-");
    const char *opts[] = {"MULTILINE=NO", NULL};
    printf("%s\n", crs ? proj_as_wkt(ctx, crs, PJ_WKT2_2019, opts) : "(null)");
    PJ_PARAM_DESCRIPTION param = {NULL, NULL, NULL, 1.5, NULL, 1.0, PJ_UT_SCALE};
    PJ *tr = proj_create_transformation(ctx, NULL, NULL, NULL, geog, geog, NULL, NULL, NULL, NULL, 1, &param, -1);
    printf("transformation %d | %s\n", tr != NULL, tr ? proj_as_wkt(ctx, tr, PJ_WKT2_2019, opts) : "-");
    return 0;
}
