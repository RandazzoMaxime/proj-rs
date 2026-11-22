/* An operation of proj_create_operations that PROJ can only write as a PROJ string is a
   PROJBasedOperation (coordinateoperationfactory.cpp, createOperationsCompoundToGeog): type 24,
   no step, method "PROJ-based operation method (approximate): <pipeline>", WKT keyword
   COORDINATEOPERATION and PROJJSON type Transformation. run.sh compares with libproj 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <string.h>

static void keyword(const char *text, const char *after, char *out, size_t size) {
    out[0] = '\0';
    if (!text) return;
    const char *start = after ? strstr(text, after) : text;
    if (!start) return;
    start += after ? strlen(after) : 0;
    size_t n = strcspn(start, after ? "\"" : "[");
    if (n >= size) n = size - 1;
    memcpy(out, start, n);
    out[n] = '\0';
}

static void show(PJ_CONTEXT *ctx, const char *source, const char *target) {
    PJ_OPERATION_FACTORY_CONTEXT *factory = proj_create_operation_factory_context(ctx, NULL);
    proj_operation_factory_context_set_spatial_criterion(ctx, factory,
        PROJ_SPATIAL_CRITERION_PARTIAL_INTERSECTION);
    proj_operation_factory_context_set_grid_availability_use(ctx, factory,
        PROJ_GRID_AVAILABILITY_KNOWN_AVAILABLE);
    PJ *from = proj_create(ctx, source), *to = proj_create(ctx, target);
    PJ_OBJ_LIST *list = proj_create_operations(ctx, from, to, factory);
    int count = proj_list_get_count(list);
    printf("%s -> %s: %d operations\n", source, target, count);
    for (int i = 0; i < count; i++) {
        PJ *op = proj_list_get(ctx, list, i);
        const char *method = NULL;
        proj_coordoperation_get_method_info(ctx, op, &method, NULL, NULL);
        const char *wkt = proj_as_wkt(ctx, op, PJ_WKT2_2019, NULL);
        const char *json = proj_as_projjson(ctx, op, NULL);
        char wkt_keyword[40], wkt_method[400], json_type[60];
        keyword(wkt, NULL, wkt_keyword, sizeof wkt_keyword);
        keyword(wkt, "METHOD[\"", wkt_method, sizeof wkt_method);
        keyword(json, "\"type\": \"", json_type, sizeof json_type);
        PJ *step = proj_concatoperation_get_step(ctx, op, 0);
        printf("  op%d type=%d steps=%d step0=%s params=%d grids=%d\n", i, proj_get_type(op),
               proj_concatoperation_get_step_count(ctx, op), step ? "yes" : "null",
               proj_coordoperation_get_param_count(ctx, op),
               proj_coordoperation_get_grid_used_count(ctx, op));
        printf("    method=%s\n    wkt=%s method=%s\n    json=%s\n", method ? method : "null",
               wkt_keyword, wkt_method, json_type);
        proj_destroy(step);
        proj_destroy(op);
    }
    proj_list_destroy(list);
    proj_destroy(from);
    proj_destroy(to);
    proj_operation_factory_context_destroy(factory);
}

int main(void) {
    PJ_CONTEXT *ctx = proj_context_create();
    proj_log_level(ctx, PJ_LOG_NONE);
    show(ctx, "EPSG:4326+3855", "EPSG:4979");
    show(ctx, "EPSG:4326+5773", "EPSG:4979");
    show(ctx, "EPSG:4326", "EPSG:2154");
    proj_context_destroy(ctx);
    return 0;
}
