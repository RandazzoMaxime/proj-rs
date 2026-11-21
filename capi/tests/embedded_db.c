/* Creates EPSG:32631 after proj_context_set_search_paths(argv[1]) when an argument is given, and
   prints ok or fail. run.sh runs it against the C library of proj-rs, which reads its built-in
   proj.db, and against libproj of PROJ 9.9.0, which looks for proj.db on the search path. */
#include <proj.h>
#include <stdio.h>

int main(int argc, char **argv) {
    PJ_CONTEXT *ctx = proj_context_create();
    if (argc > 1) {
        const char *paths[1];
        paths[0] = argv[1];
        proj_context_set_search_paths(ctx, 1, paths);
    }
    PJ *crs = proj_create(ctx, "EPSG:32631");
    printf("%s\n", crs ? "ok" : "fail");
    proj_destroy(crs);
    proj_context_destroy(ctx);
    return 0;
}
