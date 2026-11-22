/* For each pair of CRS given as arguments (source target ...), prints proj_pj_info().definition of
   proj_create_crs_to_crs, then of its proj_normalize_for_visualization. run.sh compares the
   output of the C library of proj-rs with the one of libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>

int main(int argc, char **argv) {
    for (int i = 1; i + 1 < argc; i += 2) {
        PJ *op = proj_create_crs_to_crs(NULL, argv[i], argv[i + 1], NULL);
        printf("%s %s\n", argv[i], argv[i + 1]);
        if (op == NULL) {
            printf("  null\n");
            continue;
        }
        PJ *normalized = proj_normalize_for_visualization(NULL, op);
        printf("  %s\n", proj_pj_info(op).definition);
        printf("  %s\n", normalized ? proj_pj_info(normalized).definition : "(null)");
        proj_destroy(normalized);
        proj_destroy(op);
    }
    return 0;
}
