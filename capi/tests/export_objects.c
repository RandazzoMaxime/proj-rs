/* proj_as_wkt (WKT2:2019, WKT1_GDAL), proj_as_projjson and proj_get_name of objects that are not
   CRS nor operations: ellipsoids, prime meridians, datums of every kind, datum ensembles and
   coordinate systems. run.sh compares the output with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>

static void show(const char *label, PJ *object) {
    printf("== %s\n", label);
    if (!object) {
        printf("null\n");
        return;
    }
    const char *text = proj_get_name(object);
    printf("type=%d name=%s\n", (int)proj_get_type(object), text ? text : "(null)");
    text = proj_as_wkt(NULL, object, PJ_WKT2_2019, NULL);
    printf("%s\n", text ? text : "(no WKT2)");
    text = proj_as_wkt(NULL, object, PJ_WKT1_GDAL, NULL);
    printf("%s\n", text ? text : "(no WKT1)");
    text = proj_as_projjson(NULL, object, NULL);
    printf("%s\n", text ? text : "(no PROJJSON)");
    proj_destroy(object);
}

static PJ *from_db(const char *code, PJ_CATEGORY category) {
    return proj_create_from_database(NULL, "EPSG", code, category, 0, NULL);
}

static PJ *from_wkt(const char *wkt) {
    return proj_create_from_wkt(NULL, wkt, NULL, NULL, NULL);
}

int main(void) {
    show("ellipsoid 7030", from_db("7030", PJ_CATEGORY_ELLIPSOID));
    show("prime meridian 8903", from_db("8903", PJ_CATEGORY_PRIME_MERIDIAN));
    show("prime meridian 8901", from_db("8901", PJ_CATEGORY_PRIME_MERIDIAN));
    show("datum 6171", from_db("6171", PJ_CATEGORY_DATUM));
    show("vertical datum 5101", from_db("5101", PJ_CATEGORY_DATUM));
    show("ensemble 6326", from_db("6326", PJ_CATEGORY_DATUM_ENSEMBLE));
    show("engineering datum", from_wkt("ENGINEERINGDATUM[\"Engineering datum\"]"));
    show("parametric datum", from_wkt("PDATUM[\"Mean Sea Level\",ANCHOR[\"1013.25 hPa\"]]"));
    show("temporal datum", from_wkt("TDATUM[\"Gregorian calendar\",CALENDAR[\"proleptic "
                                    "Gregorian\"],TIMEORIGIN[0000-01-01]]"));
    PJ *geographic = proj_create(NULL, "EPSG:4326");
    PJ *projected = proj_create(NULL, "EPSG:32631");
    show("cs of 4326", proj_crs_get_coordinate_system(NULL, geographic));
    show("cs of 32631", proj_crs_get_coordinate_system(NULL, projected));
    show("ellipsoid of 4326", proj_get_ellipsoid(NULL, geographic));
    show("prime meridian of 4326", proj_get_prime_meridian(NULL, geographic));
    proj_destroy(geographic);
    proj_destroy(projected);
    return 0;
}
