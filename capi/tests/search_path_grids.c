/* The grids of a context whose search paths come from proj_context_set_search_paths (pyproj's
   append_data_dir), not from PROJ_DATA: proj_coordoperation_get_grid_used and
   proj_coordoperation_is_instantiable look for the files there (lookForGridInfo opens them
   through the context, c_api.cpp:7552, 7925). Arguments: the directory of proj.db, a directory
   of grids, then pairs of CRS. run.sh compares the output with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <string.h>

static void list(const char *data, const char *grids, const char *source, const char *target) {
    PJ_CONTEXT *ctx = proj_context_create();
    const char *paths[] = {data, grids};
    proj_context_set_search_paths(ctx, 2, paths);
    PJ *from = proj_create(ctx, source), *to = proj_create(ctx, target);
    PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(ctx, NULL);
    proj_operation_factory_context_set_spatial_criterion(
        ctx, factory, PROJ_SPATIAL_CRITERION_PARTIAL_INTERSECTION);
    proj_operation_factory_context_set_grid_availability_use(ctx, factory,
                                                             PROJ_GRID_AVAILABILITY_IGNORED);
    PJ_OBJ_LIST *operations = proj_create_operations(ctx, from, to, factory);
    int count = operations ? proj_list_get_count(operations) : 0;
    printf("%s -> %s: operations=%d\n", source, target, count);
    for (int i = 0; i < count; i++) {
        PJ *operation = proj_list_get(ctx, operations, i);
        printf("  instantiable=%d %s\n", proj_coordoperation_is_instantiable(ctx, operation),
               proj_get_name(operation));
        int used = proj_coordoperation_get_grid_used_count(ctx, operation);
        for (int grid = 0; grid < used; grid++) {
            const char *short_name, *full_name, *package, *url;
            int direct, open, available;
            proj_coordoperation_get_grid_used(ctx, operation, grid, &short_name, &full_name,
                                              &package, &url, &direct, &open, &available);
            const char *base = strrchr(full_name, '/');
            printf("    %s available=%d file=%s\n", short_name, available,
                   base ? base + 1 : full_name);
        }
        proj_destroy(operation);
    }
    proj_list_destroy(operations);
    proj_operation_factory_context_destroy(factory);
    proj_destroy(from);
    proj_destroy(to);
    proj_context_destroy(ctx);
}

int main(int argc, char **argv) {
    for (int i = 3; i + 1 < argc; i += 2) list(argv[1], argv[2], argv[i], argv[i + 1]);
    return 0;
}
