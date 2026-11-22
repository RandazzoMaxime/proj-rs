/* proj_create on the operations PROJ's gie suite expects to fail (invalid_operations.txt): PROJ
   instantiates a PROJ string before it makes the object (io.cpp:12799), so an operation pj_init
   refuses gives NULL. run.sh compares object or NULL with libproj 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    FILE *list = fopen(argc > 1 ? argv[1] : "invalid_operations.txt", "r");
    if (!list) return 1;
    char line[4096];
    while (fgets(line, sizeof line, list)) {
        line[strcspn(line, "\n")] = '\0';
        if (line[0] == '\0' || line[0] == '#') continue;
        PJ_CONTEXT *ctx = proj_context_create();
        proj_log_level(ctx, PJ_LOG_NONE);
        PJ *object = proj_create(ctx, line);
        printf("%s -> %s\n", line, object ? "object" : "null");
        proj_destroy(object);
        proj_context_destroy(ctx);
    }
    fclose(list);
    return 0;
}
