/* "export": the PROJ strings (PROJ.5 and PROJ.4) of every CRS of the EPSG, ESRI and IGNF
 * authorities. "create": proj_create of each line of stdin after a refused PROJ string. */
#include <stdio.h>
#include <string.h>
#include <proj.h>

static void export_strings(PJ_CONTEXT *ctx) {
  const PJ_TYPE types[] = {PJ_TYPE_PROJECTED_CRS, PJ_TYPE_GEOGRAPHIC_2D_CRS,
                           PJ_TYPE_GEOGRAPHIC_3D_CRS, PJ_TYPE_GEOCENTRIC_CRS,
                           PJ_TYPE_VERTICAL_CRS, PJ_TYPE_COMPOUND_CRS};
  const char *authorities[] = {"EPSG", "ESRI", "IGNF"};
  for (int a = 0; a < 3; a++) {
    for (int t = 0; t < 6; t++) {
      PROJ_STRING_LIST codes = proj_get_codes_from_database(ctx, authorities[a], types[t], 0);
      for (char **code = codes; code && *code; code++) {
        PJ *crs = proj_create_from_database(ctx, authorities[a], *code, PJ_CATEGORY_CRS, 0, NULL);
        if (!crs) continue;
        const PJ_PROJ_STRING_TYPE kinds[] = {PJ_PROJ_5, PJ_PROJ_4};
        for (int k = 0; k < 2; k++) {
          const char *text = proj_as_proj_string(ctx, crs, kinds[k], NULL);
          if (text) printf("%s\n", text);
        }
        proj_destroy(crs);
      }
      proj_string_list_destroy(codes);
    }
  }
}

int main(int argc, char **argv) {
  PJ_CONTEXT *ctx = proj_context_create();
  proj_log_level(ctx, PJ_LOG_NONE);
  if (argc > 1 && strcmp(argv[1], "export") == 0) {
    export_strings(ctx);
  } else {
    char line[8192];
    while (fgets(line, sizeof line, stdin)) {
      line[strcspn(line, "\n")] = 0;
      proj_destroy(proj_create(ctx, "+proj=bogus"));
      PJ *object = proj_create(ctx, line);
      printf("%s -> %s type=%d errno=%d\n", line, object ? "obj" : "NULL",
             object ? (int)proj_get_type(object) : -1, proj_context_errno(ctx));
      proj_destroy(object);
    }
  }
  proj_context_destroy(ctx);
  return 0;
}
