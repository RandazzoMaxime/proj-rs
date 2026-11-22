/* proj_coordoperation_get_grid_used of operations read from the database, with and without the
   PROJ alternative grid names: lookForGridInfo finds a grid by its PROJ name or its old PROJ
   name only (factory.cpp:3452-3467), so an official name keeps an empty URL. run.sh compares the
   output with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>

int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        for (int alternative = 0; alternative < 2; alternative++) {
            PJ *operation = proj_create_from_database(NULL, "EPSG", argv[i],
                                                      PJ_CATEGORY_COORDINATE_OPERATION,
                                                      alternative, NULL);
            int count = operation ? proj_coordoperation_get_grid_used_count(NULL, operation) : -1;
            printf("EPSG:%s alternative=%d grids=%d\n", argv[i], alternative, count);
            for (int grid = 0; grid < count; grid++) {
                const char *short_name, *full_name, *package, *url;
                int direct, open, available;
                proj_coordoperation_get_grid_used(NULL, operation, grid, &short_name, &full_name,
                                                  &package, &url, &direct, &open, &available);
                printf("  %s package=%s url=%s direct=%d open=%d found=%s\n", short_name, package,
                       url, direct, open, full_name[0] ? "yes" : "no");
            }
            proj_destroy(operation);
        }
    }
    return 0;
}
