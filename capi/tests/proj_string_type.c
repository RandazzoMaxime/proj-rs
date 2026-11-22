/* proj_create on a PROJ string: without +type=crs it is a PROJ-based coordinate operation
   (PJ_TYPE_OTHER_COORDINATE_OPERATION), with it a CRS, as PROJ's createFromUserInput reads it.
   pyproj's Proj relies on that type. Run against two libraries, the outputs must be identical. */
#include <stdio.h>

#include "proj.h"

int main(void) {
    const char *definitions[] = {
        "+proj=longlat +datum=WGS84 +no_defs",
        "+proj=longlat +datum=WGS84 +type=crs",
        "+proj=utm +zone=31 +ellps=WGS84",
        "+proj=utm +zone=31 +ellps=WGS84 +type=crs",
        "+proj=geocent +datum=WGS84",
        "+proj=merc",
        "+proj=etmerc +lat_0=0 +lon_0=3",
        "proj=longlat datum=WGS84",
        "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=merc",
        "EPSG:4326",
        NULL};
    PJ_CONTEXT *ctx = proj_context_create();
    for (int i = 0; definitions[i]; i++) {
        PJ *object = proj_create(ctx, definitions[i]);
        printf("%s | type %d | crs %d\n", definitions[i], object ? (int)proj_get_type(object) : -1,
               object ? proj_is_crs(object) : -1);
        proj_destroy(object);
    }
    proj_context_destroy(ctx);
    return 0;
}
