/* Prints the user-writable directory and the search path the library reports, then the search
   path after proj_context_set_search_paths(argv[1]) when an argument is given. run.sh runs it
   against the C library of proj-rs and against libproj of PROJ 9.9.0, under the same environment. */
#include <proj.h>
#include <stdio.h>

int main(int argc, char **argv) {
    printf("%s\n", proj_context_get_user_writable_directory(NULL, 0));
    printf("%s\n", proj_info().searchpath);
    if (argc > 1) {
        const char *paths[1];
        paths[0] = argv[1];
        proj_context_set_search_paths(NULL, 1, paths);
        printf("%s\n", proj_info().searchpath);
    }
    return 0;
}
