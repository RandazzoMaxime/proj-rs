/* proj_create on the operations PROJ's gie suite expects to fail (invalid_operations.txt): PROJ
   instantiates a PROJ string before it makes the object (io.cpp:12799), so an operation pj_init
   refuses gives NULL, with the code pj_init leaves and the line proj_create logs. run.sh compares
   the object or NULL, the error code and the last error line with libproj 9.9.0. */
#include <proj.h>
#include <stdio.h>
#include <string.h>

static char last[2048];

static void keep_error(void *data, int level, const char *message) {
    (void)data;
    if (level == PJ_LOG_ERROR) snprintf(last, sizeof last, "%s", message);
}

int main(int argc, char **argv) {
    FILE *list = fopen(argc > 1 ? argv[1] : "invalid_operations.txt", "r");
    if (!list) return 1;
    char line[4096];
    while (fgets(line, sizeof line, list)) {
        line[strcspn(line, "\n")] = '\0';
        if (line[0] == '\0' || line[0] == '#') continue;
        PJ_CONTEXT *ctx = proj_context_create();
        proj_log_func(ctx, NULL, keep_error);
        last[0] = '\0';
        PJ *object = proj_create(ctx, line);
        printf("%s -> %s errno=%d | %s\n", line, object ? "object" : "null",
               proj_context_errno(ctx), last);
        proj_destroy(object);
        proj_context_destroy(ctx);
    }
    fclose(list);
    return 0;
}
