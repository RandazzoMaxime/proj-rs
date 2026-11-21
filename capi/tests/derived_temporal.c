/* A derived geographic CRS (rotated pole, as pyproj's CF reader builds it) and a temporal CRS,
   read from PROJJSON or WKT: proj_get_type follows the C++ class (a DerivedGeographicCRS is a
   GeographicCRS, c_api.cpp:1281-1319), and the accessors reach the coordinate system, its axes,
   the ellipsoid, the datum, the base and the deriving conversion. run.sh compares the output
   with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <stdlib.h>

static char *slurp(const char *path) {
    FILE *file = fopen(path, "rb");
    if (!file) return NULL;
    fseek(file, 0, SEEK_END);
    long size = ftell(file);
    rewind(file);
    char *text = malloc((size_t)size + 1);
    size_t got = fread(text, 1, (size_t)size, file);
    text[got] = 0;
    fclose(file);
    return text;
}

static const char *name_of(PJ *object) {
    const char *name = object ? proj_get_name(object) : NULL;
    return object ? (name ? name : "(no name)") : "null";
}

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        char *text = slurp(argv[i]);
        PJ *crs = text ? proj_create(NULL, text) : NULL;
        printf("%s: type=%d\n", argv[i], crs ? (int)proj_get_type(crs) : -1);
        free(text);
        if (!crs) continue;
        PJ *cs = proj_crs_get_coordinate_system(NULL, crs);
        int count = cs ? proj_cs_get_axis_count(NULL, cs) : -1;
        printf("  cs type=%d axes=%d\n", cs ? (int)proj_cs_get_type(NULL, cs) : -1, count);
        for (int axis = 0; axis < count; axis++) {
            const char *name, *abbreviation, *direction, *unit;
            double factor;
            proj_cs_get_axis_info(NULL, cs, axis, &name, &abbreviation, &direction, &factor, &unit,
                                  NULL, NULL);
            printf("    %s (%s) %s %s %g\n", name, abbreviation, direction, unit, factor);
        }
        PJ *conversion = proj_crs_get_coordoperation(NULL, crs);
        printf("  ellipsoid=%s datum=%s geodetic=%s base=%s\n",
               name_of(proj_get_ellipsoid(NULL, crs)), name_of(proj_crs_get_datum_forced(NULL, crs)),
               name_of(proj_crs_get_geodetic_crs(NULL, crs)), name_of(proj_get_source_crs(NULL, crs)));
        printf("  conversion=%s parameters=%d\n", name_of(conversion),
               conversion ? proj_coordoperation_get_param_count(NULL, conversion) : -1);
        proj_destroy(conversion);
        proj_destroy(cs);
        proj_destroy(crs);
    }
    return 0;
}
