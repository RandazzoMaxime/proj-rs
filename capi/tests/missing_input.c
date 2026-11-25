/* What the constructors of iso19111/c_api.cpp do with a NULL argument and with an argument of the
   wrong type: object or NULL, errno, and the last line logged. PROJ 9.9.0 answers a NULL with
   "missing required input" and PROJ_ERR_OTHER_API_MISUSE in some of them, "Missing input
   datum_or_datum_ensemble" in others, and a wrong type with NULL and nothing logged, or with its
   own message. The NULLs PROJ dereferences (proj_create_derived_geographic_crs, the coordinate
   system of proj_create_geographic_crs_from_datum) are not called. run.sh compares the output
   with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <string.h>

static char last[1024];

static void logger(void *data, int level, const char *message) {
    (void)data;
    (void)level;
    snprintf(last, sizeof last, "%s", message);
}

static PJ_CONTEXT *ctx, *call;

/* Each case runs on a fresh context: its errno starts at 0. */
static void begin(void) {
    call = proj_context_create();
    proj_log_func(call, NULL, logger);
    last[0] = 0;
}

static void report(const char *what, PJ *made) {
    printf("%s: %s errno=%d log=\"%s\"\n", what, made ? "object" : "NULL", proj_context_errno(call),
           last);
    proj_destroy(made);
    proj_context_destroy(call);
}

static void report_list(const char *what, PJ_OBJ_LIST *made) {
    printf("%s: %s errno=%d log=\"%s\"\n", what, made ? "list" : "NULL", proj_context_errno(call),
           last);
    proj_list_destroy(made);
    proj_context_destroy(call);
}

int main(void) {
    ctx = proj_context_create();
    PJ *geog = proj_create(ctx, "EPSG:4326");
    PJ *geog3d = proj_create(ctx, "EPSG:4979");
    PJ *projected = proj_create(ctx, "EPSG:32631");
    PJ *vertical = proj_create(ctx, "EPSG:5703");
    PJ *conversion = proj_crs_get_coordoperation(ctx, projected);
    PJ *cartesian = proj_crs_get_coordinate_system(ctx, projected);
    PJ *ellipsoidal = proj_crs_get_coordinate_system(ctx, geog);
    PJ *datum = proj_crs_get_datum_forced(ctx, geog);
    PJ *ellipsoid = proj_get_ellipsoid(ctx, geog);
    PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(ctx, NULL);

    begin(); report("projected NULL base", proj_create_projected_crs(call, "x", NULL, conversion, cartesian));
    begin(); report("projected NULL conversion", proj_create_projected_crs(call, "x", geog, NULL, cartesian));
    begin(); report("projected NULL cs", proj_create_projected_crs(call, "x", geog, conversion, NULL));
    begin(); report("projected base ellipsoid", proj_create_projected_crs(call, "x", ellipsoid, conversion, cartesian));
    begin(); report("projected conversion ellipsoid", proj_create_projected_crs(call, "x", geog, ellipsoid, cartesian));
    begin(); report("projected cs ellipsoidal", proj_create_projected_crs(call, "x", geog, conversion, ellipsoidal));
    begin(); report("projected valid", proj_create_projected_crs(call, "x", geog, conversion, cartesian));

    begin(); report("derived projected NULL base", proj_create_derived_projected_crs(call, "x", NULL, conversion, cartesian));
    begin(); report("derived projected NULL conversion", proj_create_derived_projected_crs(call, "x", projected, NULL, cartesian));
    begin(); report("derived projected NULL cs", proj_create_derived_projected_crs(call, "x", projected, conversion, NULL));
    begin(); report("derived projected base geographic", proj_create_derived_projected_crs(call, "x", geog, conversion, cartesian));
    begin(); report("derived projected conversion ellipsoid", proj_create_derived_projected_crs(call, "x", projected, ellipsoid, cartesian));
    begin(); report("derived projected cs ellipsoidal", proj_create_derived_projected_crs(call, "x", projected, conversion, ellipsoidal));

    begin(); report("derived geographic base ellipsoid", proj_create_derived_geographic_crs(call, "x", ellipsoid, conversion, ellipsoidal));
    begin(); report("derived geographic base projected", proj_create_derived_geographic_crs(call, "x", projected, conversion, ellipsoidal));
    begin(); report("derived geographic conversion ellipsoid", proj_create_derived_geographic_crs(call, "x", geog, ellipsoid, ellipsoidal));
    begin(); report("derived geographic cs cartesian", proj_create_derived_geographic_crs(call, "x", geog, conversion, cartesian));

    begin(); report("geographic from datum NULL datum", proj_create_geographic_crs_from_datum(call, "x", NULL, ellipsoidal));
    begin(); report("geographic from datum datum ellipsoid", proj_create_geographic_crs_from_datum(call, "x", ellipsoid, ellipsoidal));
    begin(); report("geographic from datum cs cartesian", proj_create_geographic_crs_from_datum(call, "x", datum, cartesian));
    begin(); report("geographic from datum valid", proj_create_geographic_crs_from_datum(call, "x", datum, ellipsoidal));

    begin(); report("geocentric from datum NULL datum", proj_create_geocentric_crs_from_datum(call, "x", NULL, NULL, 0));
    begin(); report("geocentric from datum datum ellipsoid", proj_create_geocentric_crs_from_datum(call, "x", ellipsoid, NULL, 0));
    begin(); report("geocentric from datum valid", proj_create_geocentric_crs_from_datum(call, "x", datum, NULL, 0));

    begin(); report("bound vertical NULL vertical", proj_crs_create_bound_vertical_crs(call, NULL, geog3d, "g.gtx"));
    begin(); report("bound vertical NULL hub", proj_crs_create_bound_vertical_crs(call, vertical, NULL, "g.gtx"));
    begin(); report("bound vertical NULL grid", proj_crs_create_bound_vertical_crs(call, vertical, geog3d, NULL));
    begin(); report("bound vertical vertical geographic", proj_crs_create_bound_vertical_crs(call, geog, geog3d, "g.gtx"));
    begin(); report("bound vertical hub ellipsoid", proj_crs_create_bound_vertical_crs(call, vertical, ellipsoid, "g.gtx"));
    begin(); report("bound vertical valid", proj_crs_create_bound_vertical_crs(call, vertical, geog3d, "g.gtx"));

    begin(); report_list("operations NULL source", proj_create_operations(call, NULL, geog, factory));
    begin(); report_list("operations NULL target", proj_create_operations(call, geog, NULL, factory));
    begin(); report_list("operations NULL context", proj_create_operations(call, geog, projected, NULL));
    begin(); report_list("operations source ellipsoid", proj_create_operations(call, ellipsoid, geog, factory));
    begin(); report_list("operations target ellipsoid", proj_create_operations(call, geog, ellipsoid, factory));

    proj_operation_factory_context_destroy(factory);
    PJ *all[] = {geog, geog3d, projected, vertical, conversion, cartesian, ellipsoidal, datum,
                 ellipsoid};
    for (size_t i = 0; i < sizeof all / sizeof all[0]; i++) proj_destroy(all[i]);
    proj_context_destroy(ctx);
    return 0;
}
