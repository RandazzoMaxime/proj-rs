/* The deriving conversion of a projected or derived projected CRS: PROJ's
   DerivedCRS::derivingConversion sets the base as source and the CRS itself as target, so the
   conversion is instantiable and exports to a PROJ string. run.sh compares the output with
   libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>

static const char *name_of(PJ *object) {
    const char *name = object ? proj_get_name(object) : NULL;
    return object ? (name ? name : "(no name)") : "null";
}

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        PJ *crs = proj_create(NULL, argv[i]);
        PJ *conversion = crs ? proj_crs_get_coordoperation(NULL, crs) : NULL;
        printf("%s: conversion=%s\n", argv[i], name_of(conversion));
        if (conversion) {
            PJ *source = proj_get_source_crs(NULL, conversion);
            PJ *target = proj_get_target_crs(NULL, conversion);
            const char *text = proj_as_proj_string(NULL, conversion, PJ_PROJ_5, NULL);
            printf("  instantiable=%d source=%s target=%s\n  %s\n",
                   proj_coordoperation_is_instantiable(NULL, conversion), name_of(source),
                   name_of(target), text ? text : "-");
            proj_destroy(source);
            proj_destroy(target);
        }
        proj_destroy(conversion);
        proj_destroy(crs);
    }
    return 0;
}
