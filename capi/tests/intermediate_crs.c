/* proj_operation_factory_context_set_allowed_intermediate_crs reads its list up to the NULL that
   ends it, and not one slot further: the slot after the NULL holds an invalid pointer here, which
   a read past the end would follow (pyproj's TransformerGroup crashed on it). Run against two
   libraries, the outputs must be identical. */
#include <stdio.h>

#include "proj.h"

int main(void) {
    PJ_CONTEXT *ctx = proj_context_create();
    const char *list[] = {"EPSG", "4326", "EPSG", "4979", NULL, (const char *)1};
    PJ *source = proj_create(ctx, "EPSG:4230");
    PJ *target = proj_create(ctx, "EPSG:4326");
    PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(ctx, NULL);
    if (!source || !target || !factory) {
        printf("creation failed\n");
        return 1;
    }
    proj_operation_factory_context_set_allow_use_intermediate_crs(
        ctx, factory, PROJ_INTERMEDIATE_CRS_USE_ALWAYS);
    proj_operation_factory_context_set_allowed_intermediate_crs(ctx, factory, list);
    PJ_OBJ_LIST *operations = proj_create_operations(ctx, source, target, factory);
    int count = operations ? proj_list_get_count(operations) : -1;
    printf("operations %d\n", count);
    for (int i = 0; i < count && i < 3; i++) {
        PJ *operation = proj_list_get(ctx, operations, i);
        printf("%d %s\n", i, proj_get_name(operation));
        proj_destroy(operation);
    }
    proj_list_destroy(operations);
    proj_operation_factory_context_destroy(factory);
    proj_destroy(source);
    proj_destroy(target);
    proj_context_destroy(ctx);
    return 0;
}
