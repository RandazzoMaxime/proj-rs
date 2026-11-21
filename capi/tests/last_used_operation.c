/* proj_trans_get_last_used_operation before and after the first point: NULL until proj_trans has
   run, whatever the object holds (trans.cpp:348 sets iCurCoordOp, :371 tests it), then the
   operation used. For each definition (a pair "source target", or one PROJ string): before, then
   after a point. run.sh compares the output with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <string.h>

static void report(const char *when, PJ *op) {
    PJ *last = proj_trans_get_last_used_operation(op);
    const char *name = last ? proj_get_name(last) : NULL;
    printf("  %s: %s\n", when, last ? (name ? name : "(no name)") : "null");
    proj_destroy(last);
}

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        PJ *op;
        if (!strcmp(argv[i], "-p") && i + 1 < argc) {
            op = proj_create(NULL, argv[++i]);
            printf("%s\n", argv[i]);
        } else if (i + 1 < argc) {
            op = proj_create_crs_to_crs(NULL, argv[i], argv[i + 1], NULL);
            printf("%s %s\n", argv[i], argv[i + 1]);
            i++;
        } else {
            break;
        }
        if (!op) {
            printf("  null\n");
            continue;
        }
        report("before", op);
        PJ_COORD out = proj_trans(op, PJ_FWD, proj_coord(49, 2, 0, 0));
        (void)out;
        report("after", op);
        proj_destroy(op);
    }
    return 0;
}
