/* How often a context logs "Cannot open <url>: <error>" for a grid of the CDN that does not open,
   over a series of calls on that context. PROJ keeps the answers of lookForGridInfo in the
   DatabaseContext of the context, an LRU of 128 keys that grows to 138 before it is pruned back
   (cacheGridInfo_, factory.cpp:860-871, 3398-3539): a grid that did not open is not tried again
   while its key is kept, and the keys of the sort, of the grid rejection (network off), of
   proj_coordoperation_get_grid_used_count and of the lookup open_resource_file makes before a
   network open share that cache, so the count of each call depends on the calls before it.
   The series are those of terra's proj_pipelines (R) and of its test_project.R. run.sh compares
   the output with libproj of PROJ 9.9.0 built without libcurl, whose every network open answers
   "Network functionality not available". */
#include <math.h>
#include <proj.h>
#include <stdio.h>
#include <string.h>

static int cannot_open;

static void logger(void *data, int level, const char *message) {
    (void)data;
    (void)level;
    if (strstr(message, "Cannot open ") == message) cannot_open++;
}

static PJ_CONTEXT *network_on(PJ_CONTEXT *ctx) {
    proj_log_func(ctx, NULL, logger);
    proj_context_set_enable_network(ctx, 1);
    return ctx;
}

/* proj_create_operations alone, crs extent NONE, partial intersection. */
static void listing(PJ_CONTEXT *ctx, const char *source, const char *target,
                    PROJ_GRID_AVAILABILITY_USE grids, int area) {
    PJ *from = proj_create(ctx, source), *to = proj_create(ctx, target);
    PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(ctx, NULL);
    proj_operation_factory_context_set_spatial_criterion(
        ctx, factory, PROJ_SPATIAL_CRITERION_PARTIAL_INTERSECTION);
    proj_operation_factory_context_set_crs_extent_use(ctx, factory, PJ_CRS_EXTENT_NONE);
    proj_operation_factory_context_set_grid_availability_use(ctx, factory, grids);
    if (area) proj_operation_factory_context_set_area_of_interest(ctx, factory, -100, 30, -90, 40);
    cannot_open = 0;
    PJ_OBJ_LIST *operations = proj_create_operations(ctx, from, to, factory);
    printf("  listing %s -> %s grids=%d area=%d: operations=%d cannot_open=%d\n", source, target,
           (int)grids, area, proj_list_get_count(operations), cannot_open);
    proj_list_destroy(operations);
    proj_operation_factory_context_destroy(factory);
    proj_destroy(from);
    proj_destroy(to);
}

/* terra's get_proj_pipelines: the listing, then each operation's definition, its grid count and
   its instantiability, on the default context. */
static void pipelines(const char *source, const char *target,
                      PROJ_GRID_AVAILABILITY_USE grids, int area) {
    cannot_open = 0;
    PJ *from = proj_create(NULL, source);
    PJ *to = from ? proj_create(NULL, target) : NULL;
    if (!from || !to) {
        printf("  pipelines %s -> %s: invalid crs cannot_open=%d\n", source, target, cannot_open);
        proj_destroy(from);
        return;
    }
    PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(NULL, NULL);
    if (area)
        proj_operation_factory_context_set_area_of_interest(NULL, factory, -100, 30, -90, 40);
    else
        proj_operation_factory_context_set_crs_extent_use(NULL, factory, PJ_CRS_EXTENT_NONE);
    proj_operation_factory_context_set_spatial_criterion(
        NULL, factory, PROJ_SPATIAL_CRITERION_PARTIAL_INTERSECTION);
    proj_operation_factory_context_set_grid_availability_use(NULL, factory, grids);
    PJ_OBJ_LIST *operations = proj_create_operations(NULL, from, to, factory);
    int count = proj_list_get_count(operations), listed = cannot_open, used = 0, instantiable = 0;
    for (int i = 0; i < count; i++) {
        PJ *operation = proj_list_get(NULL, operations, i);
        PJ_PROJ_INFO info = proj_pj_info(operation);
        if (!info.definition || !info.definition[0])
            proj_as_proj_string(NULL, operation, PJ_PROJ_5, NULL);
        used += proj_coordoperation_get_grid_used_count(NULL, operation);
        /* A second count reads the grids the object keeps: no lookup. */
        used -= proj_coordoperation_get_grid_used_count(NULL, operation);
        used += proj_coordoperation_get_grid_used_count(NULL, operation);
        instantiable += proj_coordoperation_is_instantiable(NULL, operation);
        proj_destroy(operation);
    }
    printf("  pipelines %s -> %s grids=%d area=%d: operations=%d grids_used=%d instantiable=%d "
           "cannot_open=%d (listing %d)\n",
           source, target, (int)grids, area, count, used, instantiable, cannot_open, listed);
    proj_list_destroy(operations);
    proj_operation_factory_context_destroy(factory);
    proj_destroy(from);
    proj_destroy(to);
}

/* GDAL's OGRCreateCoordinateTransformation: proj_create_crs_to_crs_from_pj with no option, the
   network on, with the only_best_default = off of the proj.ini of the data directory. PROJ asks
   KNOWN_AVAILABLE with the network on (crs_to_crs.cpp:566-573): creating the pair opens nothing,
   and the grid operations stay candidates. A candidate opens its grid at its first point only
   (defer_grid_opening, hgridshift.cpp:30-41): the points after it fail with no new attempt. */
static void pair(PJ_CONTEXT *ctx, const char *source, const char *target, const double *xy,
                 int count) {
    PJ *from = proj_create(ctx, source), *to = proj_create(ctx, target);
    cannot_open = 0;
    PJ *p = proj_create_crs_to_crs_from_pj(ctx, from, to, NULL, NULL);
    printf("  pair %s -> %s: created=%d cannot_open=%d\n", source, target, p != NULL, cannot_open);
    for (int i = 0; p && i < count; i++) {
        cannot_open = 0;
        PJ_COORD c = proj_trans(p, PJ_FWD, proj_coord(xy[2 * i], xy[2 * i + 1], 0, 0));
        int code = proj_errno(p);
        proj_errno_reset(p);
        printf("    point %.0f %.0f: %s errno=%d cannot_open=%d\n", xy[2 * i], xy[2 * i + 1],
               c.xyzt.x == HUGE_VAL ? "inf" : "finite", code, cannot_open);
    }
    proj_destroy(p);
    proj_destroy(from);
    proj_destroy(to);
}

static void grid_info(PJ_CONTEXT *ctx, const char *name) {
    int available = -1;
    cannot_open = 0;
    int found = proj_grid_get_info_from_database(ctx, name, NULL, NULL, NULL, NULL, NULL,
                                                 &available);
    printf("  grid_get_info_from_database %s: found=%d available=%d cannot_open=%d\n", name, found,
           available, cannot_open);
}

int main(void) {
    const PROJ_GRID_AVAILABILITY_USE sorting = PROJ_GRID_AVAILABILITY_USED_FOR_SORTING;
    const PROJ_GRID_AVAILABILITY_USE discard =
        PROJ_GRID_AVAILABILITY_DISCARD_OPERATION_IF_MISSING_GRID;

    printf("one context, listings in a row:\n");
    PJ_CONTEXT *ctx = network_on(proj_context_create());
    listing(ctx, "EPSG:5070", "EPSG:4326", sorting, 0);
    listing(ctx, "EPSG:5070", "EPSG:4326", discard, 0);
    listing(ctx, "EPSG:5070", "EPSG:4326", sorting, 1);
    listing(ctx, "EPSG:5070", "EPSG:4326", sorting, 0);
    listing(ctx, "EPSG:4326", "EPSG:5070", sorting, 0);
    printf("a clone starts with no record, its parent keeps its own:\n");
    PJ_CONTEXT *clone = proj_context_clone(ctx);
    proj_log_func(clone, NULL, logger);
    listing(clone, "EPSG:4326", "EPSG:5070", sorting, 0);
    listing(ctx, "EPSG:4326", "EPSG:5070", sorting, 0);
    proj_context_destroy(clone);
    proj_context_destroy(ctx);

    printf("a new context starts with no record:\n");
    ctx = network_on(proj_context_create());
    listing(ctx, "EPSG:5070", "EPSG:4326", sorting, 0);
    printf("the discarding listing first, network off for its rejection:\n");
    proj_context_destroy(ctx);
    ctx = network_on(proj_context_create());
    listing(ctx, "EPSG:5070", "EPSG:4326", discard, 0);
    listing(ctx, "EPSG:5070", "EPSG:4326", sorting, 0);
    printf("the lookup of proj_grid_get_info_from_database:\n");
    grid_info(ctx, "us_noaa_nvhpgn.tif");
    grid_info(ctx, "nz_linz_nzgd2kgrid0005.tif");
    grid_info(ctx, "nz_linz_nzgd2kgrid0005.tif");
    proj_context_destroy(ctx);

    printf("a pair GDAL builds, the network on:\n");
    ctx = network_on(proj_context_create());
    const double albers[] = {958000, 4470000, 1500000, 4470000, 2978000, 4470000,
                             1000000, 1000000, 1100000, 1000000};
    pair(ctx, "EPSG:5070", "EPSG:4326", albers, 5);
    proj_context_destroy(ctx);

    printf("terra's test_project.R, on the default context:\n");
    network_on(NULL);
    pipelines("EPSG:4326", "EPSG:32632", sorting, 0);
    pipelines("EPSG:4326", "EPSG:3857", sorting, 0);
    pipelines("EPSG:5070", "EPSG:4326", sorting, 0);
    pipelines("EPSG:5070", "EPSG:4326", discard, 0);
    pipelines("EPSG:5070", "EPSG:4326", sorting, 1);
    pipelines("INVALID", "EPSG:4326", sorting, 0);
    pipelines("EPSG:4326", "INVALID", sorting, 0);
    pipelines("EPSG:4326", "EPSG:5070", sorting, 0);
    pipelines("EPSG:4326", "EPSG:5070", sorting, 0);
    return 0;
}
