/* A string read from a derived object, used after that object is destroyed. PROJ keeps it alive
   with the parent object, and GDAL relies on it (OGRSpatialReference::GetAuthorityCode reads the
   code of a sub-CRS after destroying it). The freed blocks are refilled before the strings are
   printed, so a dangling pointer prints the filler. Run against two libraries, the outputs must be
   identical. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "proj.h"

static void refill_freed_blocks(void) {
    for (size_t size = 1; size <= 256; size++) {
        char *block = malloc(size);
        memset(block, 'Z', size);
        block[size - 1] = 0;
        /* Kept on purpose: freeing would hand the same blocks back to the next size. */
    }
}

static void report(PJ_CONTEXT *ctx, const char *definition, int index) {
    PJ *crs = proj_create(ctx, definition);
    PJ *sub = proj_crs_get_sub_crs(ctx, crs, index);
    PJ *datum = proj_crs_get_datum_forced(ctx, sub);
    const char *auth = proj_get_id_auth_name(sub, 0);
    const char *code = proj_get_id_code(sub, 0);
    const char *name = proj_get_name(sub);
    const char *datum_name = proj_get_name(datum);
    proj_destroy(datum);
    proj_destroy(sub);
    refill_freed_blocks();
    printf("%s[%d] | %s:%s | %s | %s\n", definition, index, auth ? auth : "-", code ? code : "-",
           name ? name : "-", datum_name ? datum_name : "-");
    proj_destroy(crs);
}

/* The same for an item of a list: its strings live as long as the list. */
static void report_list_item(PJ_CONTEXT *ctx, const char *code) {
    PJ *crs = proj_create_from_database(ctx, "EPSG", code, PJ_CATEGORY_CRS, 0, NULL);
    PJ_OBJ_LIST *matches = proj_identify(ctx, crs, "EPSG", NULL, NULL);
    PJ *first = matches ? proj_list_get(ctx, matches, 0) : NULL;
    const char *name = first ? proj_get_name(first) : NULL;
    const char *id = first ? proj_get_id_code(first, 0) : NULL;
    proj_destroy(first);
    refill_freed_blocks();
    printf("identify EPSG:%s | %s | %s\n", code, id ? id : "-", name ? name : "-");
    proj_list_destroy(matches);
    proj_destroy(crs);
}

int main(void) {
    PJ_CONTEXT *ctx = proj_context_create();
    report_list_item(ctx, "2154");
    report_list_item(ctx, "4326");
    report(ctx, "EPSG:9518", 0);
    report(ctx, "EPSG:9518", 1);
    report(ctx, "EPSG:5972", 0);
    report(ctx, "EPSG:5972", 1);
    proj_context_destroy(ctx);
    return 0;
}
