/* proj_create of each line of a file, on a fresh context and after a refused PROJ string: object
 * or NULL, type, errno and last error line, then proj_trans and proj_pj_info of the object. */
#include <stdio.h>
#include <string.h>
#include <proj.h>

static char last[1024];

static void keep_last(void *data, int level, const char *message) {
  (void)data;
  (void)level;
  snprintf(last, sizeof last, "%s", message);
}

static void report(PJ_CONTEXT *ctx, const char *label, const char *definition) {
  last[0] = 0;
  PJ *object = proj_create(ctx, definition);
  printf("%s [%s] -> %s type=%d errno=%d log=%s\n", label, definition, object ? "obj" : "NULL",
         object ? (int)proj_get_type(object) : -1, proj_context_errno(ctx), last);
  if (object) {
    PJ_COORD out = proj_trans(object, PJ_FWD, proj_coord(0.1, 0.1, 0, 0));
    PJ_PROJ_INFO info = proj_pj_info(object);
    printf("  trans errno=%d finite=%d info id=%s desc=%s def=%s acc=%g\n", proj_errno(object),
           out.v[0] == out.v[0] && out.v[0] < 1e300, info.id ? info.id : "(null)",
           info.description ? info.description : "(null)",
           info.definition ? info.definition : "(null)", info.accuracy);
  }
  proj_destroy(object);
}

int main(int argc, char **argv) {
  if (argc < 2) return 2;
  FILE *cases = fopen(argv[1], "r");
  if (!cases) return 2;
  char line[4096];
  while (fgets(line, sizeof line, cases)) {
    line[strcspn(line, "\n")] = 0;
    if (!line[0] || line[0] == '#') continue;
    PJ_CONTEXT *ctx = proj_context_create();
    proj_log_func(ctx, NULL, keep_last);
    proj_log_level(ctx, PJ_LOG_ERROR);
    report(ctx, "fresh", line);
    proj_destroy(proj_create(ctx, "+proj=bogus"));
    report(ctx, "dirty", line);
    proj_context_destroy(ctx);
  }
  fclose(cases);
  const char *targets[] = {"EPSG:2154", "EPSG:3857", "EPSG:4258"};
  for (int i = 0; i < 3; i++) {
    PJ_CONTEXT *ctx = proj_context_create();
    proj_log_level(ctx, PJ_LOG_NONE);
    proj_destroy(proj_create(ctx, "+proj=bogus"));
    PJ *pair = proj_create_crs_to_crs(ctx, "EPSG:4326", targets[i], NULL);
    printf("crs_to_crs EPSG:4326 %s after a refusal -> %s errno=%d\n", targets[i],
           pair ? "obj" : "NULL", proj_context_errno(ctx));
    proj_destroy(pair);
    proj_context_destroy(ctx);
  }
  return 0;
}
