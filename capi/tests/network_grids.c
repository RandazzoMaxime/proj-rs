/* A grid the database knows counts as available when the network is on: proj_coordoperation_
   is_instantiable and proj_coordoperation_get_grid_used pass proj_context_is_network_enabled as
   considerKnownGridsAsAvailable (c_api.cpp:7552, 7925), whether or not a download would work.
   run.sh compares the output with libproj of PROJ 9.9.0, network off then on. */
#include <proj.h>
#include <stdio.h>

static void list(PJ_CONTEXT *ctx, const char *source, const char *target) {
    PJ *from = proj_create(ctx, source), *to = proj_create(ctx, target);
    PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(ctx, NULL);
    proj_operation_factory_context_set_spatial_criterion(
        ctx, factory, PROJ_SPATIAL_CRITERION_PARTIAL_INTERSECTION);
    proj_operation_factory_context_set_grid_availability_use(ctx, factory,
                                                             PROJ_GRID_AVAILABILITY_IGNORED);
    PJ_OBJ_LIST *operations = proj_create_operations(ctx, from, to, factory);
    int count = operations ? proj_list_get_count(operations) : 0;
    printf("%s -> %s: network=%d operations=%d\n", source, target,
           proj_context_is_network_enabled(ctx), count);
    for (int i = 0; i < count; i++) {
        PJ *operation = proj_list_get(ctx, operations, i);
        printf("  instantiable=%d %s\n", proj_coordoperation_is_instantiable(ctx, operation),
               proj_get_name(operation));
        int grids = proj_coordoperation_get_grid_used_count(ctx, operation);
        for (int grid = 0; grid < grids; grid++) {
            const char *short_name, *full_name, *package, *url;
            int direct, open, available;
            proj_coordoperation_get_grid_used(ctx, operation, grid, &short_name, &full_name,
                                              &package, &url, &direct, &open, &available);
            printf("    %s available=%d\n", short_name, available);
        }
        proj_destroy(operation);
    }
    proj_list_destroy(operations);
    proj_operation_factory_context_destroy(factory);
    proj_destroy(from);
    proj_destroy(to);
}

int main(int argc, char **argv) {
    for (int network = 0; network < 2; network++) {
        PJ_CONTEXT *ctx = proj_context_create();
        proj_context_set_enable_network(ctx, network);
        for (int i = 1; i + 1 < argc; i += 2) list(ctx, argv[i], argv[i + 1]);
        proj_context_destroy(ctx);
    }
    return 0;
}
