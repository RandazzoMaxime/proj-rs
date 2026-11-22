/* proj_crs_get_horizontal_datum, then proj_get_ellipsoid and proj_get_prime_meridian on what it
   returns, as pyproj reads a CRS datum: PROJ gives the datum ensemble of an ensemble CRS, which has
   no ellipsoid nor prime meridian of its own. Run against two libraries, the outputs must be
   identical. */
#include <stdio.h>
#include "proj.h"
int main(void) {
    const char *definitions[] = {"EPSG:4326", "EPSG:32615", "EPSG:4258", "EPSG:2154", "EPSG:9518",
                                 "EPSG:4807", "+proj=longlat +datum=WGS84 +type=crs", "EPSG:5703", NULL};
    for (int i = 0; definitions[i]; i++) {
        PJ *c = proj_create(0, definitions[i]);
        PJ *d = c ? proj_crs_get_horizontal_datum(0, c) : NULL;
        PJ *e = d ? proj_get_ellipsoid(0, d) : NULL;
        PJ *m = d ? proj_get_prime_meridian(0, d) : NULL;
        printf("%s | %s | type %d | ellps %s | pm %s\n", definitions[i], d ? proj_get_name(d) : "-",
               d ? (int)proj_get_type(d) : -1, e ? proj_get_name(e) : "-", m ? proj_get_name(m) : "-");
        proj_destroy(e); proj_destroy(m); proj_destroy(d); proj_destroy(c);
    }
    return 0;
}
