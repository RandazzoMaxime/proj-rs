/* proj_normalize_for_visualization of objects that are not a proj_create_crs_to_crs pair.
   "-o SOURCE TARGET": the operations proj_create_operations lists, as QGIS builds a transform
   (grids ignored, partial intersection), each normalized and applied to (2, 46).
   "-d DEFINITION": the object proj_create gives. For each result: null or not, proj_errno, type,
   PROJ string. run.sh compares the output with the one of libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <string.h>

static void normalize(PJ_CONTEXT *ctx, PJ *object, int apply) {
    PJ *normalized = proj_normalize_for_visualization(ctx, object);
    const char *text = normalized ? proj_as_proj_string(ctx, normalized, PJ_PROJ_5, NULL) : NULL;
    printf("  %s errno=%d type=%d\n  %s\n", normalized ? "ok" : "null", proj_context_errno(ctx),
           normalized ? (int)proj_get_type(normalized) : -1, text ? text : "-");
    if (normalized && apply) {
        PJ_COORD out = proj_trans(normalized, PJ_FWD, proj_coord(2, 46, 0, 0));
        printf("  %.4f %.4f\n", out.xy.x, out.xy.y);
    }
    proj_destroy(normalized);
}

int main(int argc, char **argv) {
    PJ_CONTEXT *ctx = proj_context_create();
    proj_log_level(ctx, PJ_LOG_NONE);
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "-o") && i + 2 < argc) {
            PJ *source = proj_create(ctx, argv[i + 1]), *target = proj_create(ctx, argv[i + 2]);
            printf("%s -> %s\n", argv[i + 1], argv[i + 2]);
            i += 2;
            PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(ctx, NULL);
            proj_operation_factory_context_set_grid_availability_use(ctx, factory,
                                                                     PROJ_GRID_AVAILABILITY_IGNORED);
            proj_operation_factory_context_set_spatial_criterion(
                ctx, factory, PROJ_SPATIAL_CRITERION_PARTIAL_INTERSECTION);
            PJ_OBJ_LIST *list = proj_create_operations(ctx, source, target, factory);
            int count = list ? proj_list_get_count(list) : 0;
            for (int k = 0; k < count && k < 2; k++) {
                PJ *operation = proj_list_get(ctx, list, k);
                printf(" [%d] %s\n", k, proj_get_name(operation));
                normalize(ctx, operation, 1);
                proj_destroy(operation);
            }
            proj_list_destroy(list);
            proj_operation_factory_context_destroy(factory);
            proj_destroy(source);
            proj_destroy(target);
        } else if (!strcmp(argv[i], "-d") && i + 1 < argc) {
            PJ *object = proj_create(ctx, argv[++i]);
            if (object) proj_errno_reset(object);
            printf("%s\n", argv[i]);
            normalize(ctx, object, 0);
            proj_destroy(object);
        }
    }
    proj_context_destroy(ctx);
    return 0;
}
