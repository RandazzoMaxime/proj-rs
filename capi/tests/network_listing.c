/* The paths terra (R) takes through libproj with the network on and no grid on disk:
   proj_create_operations of a CRS pair with the default grid availability (used for sorting),
   the messages its logger receives when a grid of the CDN does not open, the instantiability of
   each operation network off and on, a point transformed through a pair whose best operation
   needs such a grid, the network opens proj_create_crs_to_crs attempts (none), and
   proj_list_get_count(NULL). run.sh compares the output with libproj of
   PROJ 9.9.0 built without libcurl, whose every network open answers
   "Network functionality not available". */
#include <math.h>
#include <proj.h>
#include <stdio.h>

static int logged, quiet;

static void logger(void *data, int level, const char *message) {
    (void)data;
    logged++;
    if (!quiet) printf("    log level=%d %s\n", level, message);
}

static void through(PJ_CONTEXT *ctx, PJ *pj, double a, double b) {
    logged = 0;
    PJ_COORD c = proj_coord(a, b, 0, HUGE_VAL);
    c = proj_trans(pj, PJ_FWD, c);
    int code = proj_errno(pj);
    printf("  proj_trans %.1f %.1f: x=%s y=%s messages=%d errno=%d (%s)\n", a, b,
           isinf(c.xyzt.x) ? "inf" : (isnan(c.xyzt.x) ? "nan" : "finite"),
           isinf(c.xyzt.y) ? "inf" : (isnan(c.xyzt.y) ? "nan" : "finite"), logged, code,
           code ? proj_context_errno_string(ctx, code) : "");
}

/* terra's proj_pipelines: crs extent NONE by default (use="NONE"), partial intersection. */
static void listing(PJ_CONTEXT *ctx, const char *source, const char *target,
                    PROJ_CRS_EXTENT_USE extent) {
    PJ *from = proj_create(ctx, source), *to = proj_create(ctx, target);
    PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(ctx, NULL);
    proj_operation_factory_context_set_spatial_criterion(
        ctx, factory, PROJ_SPATIAL_CRITERION_PARTIAL_INTERSECTION);
    proj_operation_factory_context_set_crs_extent_use(ctx, factory, extent);
    logged = 0;
    printf("  proj_create_operations %s -> %s extent_use=%d\n", source, target, (int)extent);
    PJ_OBJ_LIST *operations = proj_create_operations(ctx, from, to, factory);
    int count = proj_list_get_count(operations);
    /* Network off, PROJ leaves on the context the errno of an operation the sort could not
       instantiate under PJ_CRS_EXTENT_NONE, and this library does not (docs/CLIENTS-r.md): the
       errno is compared with the network on. */
    printf("  operations=%d messages=%d", count, logged);
    if (proj_context_is_network_enabled(ctx)) printf(" errno=%d", proj_context_errno(ctx));
    printf("\n");
    if (count > 1) {
        /* What proj_list_get logs, network off, for an operation it cannot instantiate is not
           compared (docs/CLIENTS-r.md); the transformation is. */
        quiet = 1;
        PJ *operation = proj_list_get(ctx, operations, 1);
        quiet = 0;
        printf("  operation 1 of the list, %s:\n", proj_get_name(operation));
        through(ctx, operation, 40, -100);
        proj_destroy(operation);
    }
    quiet = 1;
    for (int i = 0; i < count; i++) {
        PJ *operation = proj_list_get(ctx, operations, i);
        logged = 0;
        int instantiable = proj_coordoperation_is_instantiable(ctx, operation);
        printf("  %d instantiable=%d messages=%d grids=%d %s\n", i, instantiable, logged,
               proj_coordoperation_get_grid_used_count(ctx, operation), proj_get_name(operation));
        proj_destroy(operation);
    }
    quiet = 0;
    proj_list_destroy(operations);
    proj_operation_factory_context_destroy(factory);
    proj_destroy(from);
    proj_destroy(to);
}

/* A pipeline written out, as GDAL builds it from terra's project(..., pipe=). */
static void pipeline(PJ_CONTEXT *ctx, const char *definition, double lon, double lat) {
    logged = 0;
    /* One line, "proj_create: Error 1029 ...: <last error line>": its text after the step name
       is not PROJ's (docs/CLIENTS-r.md), its count is compared. */
    quiet = 1;
    PJ *pj = proj_create(ctx, definition);
    quiet = 0;
    printf("  proj_create %s: %s messages=%d errno=%d\n", definition, pj ? "object" : "NULL",
           logged, proj_context_errno(ctx));
    if (!pj) return;
    through(ctx, pj, lon, lat);
    proj_destroy(pj);
}

static void point(PJ_CONTEXT *ctx, const char *source, const char *target, double lat,
                  double lon) {
    logged = 0;
    PJ *pair = proj_create_crs_to_crs(ctx, source, target, NULL);
    printf("  proj_create_crs_to_crs %s -> %s: messages=%d errno=%d\n", source, target, logged,
           proj_context_errno(ctx));
    if (!pair) return;
    through(ctx, pair, lat, lon);
    proj_destroy(pair);
}

static int opens;

static PROJ_NETWORK_HANDLE *refuse_open(PJ_CONTEXT *ctx, const char *url, unsigned long long offset,
                                        size_t size, void *buffer, size_t *read, size_t error_size,
                                        char *error, void *data) {
    (void)ctx, (void)url, (void)offset, (void)size, (void)buffer, (void)read, (void)data;
    opens++;
    snprintf(error, error_size, "%s", "Network functionality not available");
    return NULL;
}

static void no_close(PJ_CONTEXT *ctx, PROJ_NETWORK_HANDLE *handle, void *data) {
    (void)ctx, (void)handle, (void)data;
}

static const char *no_header(PJ_CONTEXT *ctx, PROJ_NETWORK_HANDLE *handle, const char *name,
                             void *data) {
    (void)ctx, (void)handle, (void)name, (void)data;
    return NULL;
}

static size_t no_read(PJ_CONTEXT *ctx, PROJ_NETWORK_HANDLE *handle, unsigned long long offset,
                      size_t size, void *buffer, size_t error_size, char *error, void *data) {
    (void)ctx, (void)handle, (void)offset, (void)size, (void)buffer, (void)data;
    snprintf(error, error_size, "%s", "Network functionality not available");
    return 0;
}

/* proj_create_crs_to_crs with the network on asks for PROJ_GRID_AVAILABILITY_KNOWN_AVAILABLE
   (crs_to_crs.cpp:572-576): a grid proj.db knows is not opened at creation. */
static void creation_opens(const char *source, const char *target) {
    PJ_CONTEXT *ctx = proj_context_create();
    quiet = 1;
    proj_log_func(ctx, NULL, logger);
    proj_context_set_network_callbacks(ctx, refuse_open, no_close, no_header, no_read, NULL);
    proj_context_set_enable_network(ctx, 1);
    opens = 0;
    PJ *pair = proj_create_crs_to_crs(ctx, source, target, NULL);
    printf("opens at proj_create_crs_to_crs %s -> %s: %d (%s)\n", source, target, opens,
           pair ? "pair" : "NULL");
    proj_destroy(pair);
    proj_context_destroy(ctx);
    quiet = 0;
}

int main(void) {
    printf("proj_list_get_count(NULL)=%d\n", proj_list_get_count(NULL));
    for (int network = 0; network < 2; network++) {
        PJ_CONTEXT *ctx = proj_context_create();
        proj_log_func(ctx, NULL, logger);
        proj_context_set_enable_network(ctx, network);
        printf("network=%d\n", proj_context_is_network_enabled(ctx));
        listing(ctx, "EPSG:4326", "EPSG:5070", PJ_CRS_EXTENT_SMALLEST);
        listing(ctx, "EPSG:4326", "EPSG:5070", PJ_CRS_EXTENT_NONE);
        point(ctx, "EPSG:4267", "EPSG:4326", 40, -100);
        point(ctx, "EPSG:4326", "EPSG:5070", 40, -100);
        pipeline(ctx,
                 "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert "
                 "+xy_in=deg +xy_out=rad +step +inv +proj=hgridshift +grids=us_noaa_kshpgn.tif "
                 "+step +proj=aea +lat_0=23 +lon_0=-96 +lat_1=29.5 +lat_2=45.5 +x_0=0 +y_0=0 "
                 "+ellps=GRS80",
                 38.5, -98);
        proj_context_destroy(ctx);
    }
    creation_opens("EPSG:4267", "EPSG:4326");
    creation_opens("EPSG:4326", "EPSG:5070");
    creation_opens("EPSG:4269", "EPSG:6318");
    return 0;
}
