/* "export": the WKT2:2019 and WKT1 GDAL of every CRS and coordinate operation of the EPSG
 * database, one per line. "check": proj_create_from_wkt of each line of stdin, with its warnings
 * and grammar errors. */
#include <stdio.h>
#include <string.h>
#include <proj.h>

static void export_wkt(PJ_CONTEXT *ctx) {
  const PJ_TYPE types[] = {PJ_TYPE_PROJECTED_CRS, PJ_TYPE_GEOGRAPHIC_2D_CRS, PJ_TYPE_COMPOUND_CRS,
                           PJ_TYPE_GEOCENTRIC_CRS, PJ_TYPE_CONVERSION, PJ_TYPE_TRANSFORMATION};
  const char *options[] = {"MULTILINE=NO", NULL};
  for (int t = 0; t < 6; t++) {
    PROJ_STRING_LIST codes = proj_get_codes_from_database(ctx, "EPSG", types[t], 0);
    for (char **code = codes; code && *code; code++) {
      PJ_CATEGORY category = t < 4 ? PJ_CATEGORY_CRS : PJ_CATEGORY_COORDINATE_OPERATION;
      PJ *object = proj_create_from_database(ctx, "EPSG", *code, category, 0, NULL);
      if (!object) continue;
      const PJ_WKT_TYPE kinds[] = {PJ_WKT2_2019, PJ_WKT1_GDAL};
      for (int k = 0; k < 2; k++) {
        const char *text = proj_as_wkt(ctx, object, kinds[k], options);
        if (text) printf("%s\n", text);
      }
      proj_destroy(object);
    }
    proj_string_list_destroy(codes);
  }
}

int main(int argc, char **argv) {
  PJ_CONTEXT *ctx = proj_context_create();
  proj_log_level(ctx, PJ_LOG_NONE);
  if (argc > 1 && strcmp(argv[1], "export") == 0) {
    export_wkt(ctx);
  } else {
    static char line[1 << 17];
    const char *options[] = {"STRICT=NO", NULL};
    while (fgets(line, sizeof line, stdin)) {
      line[strcspn(line, "\n")] = 0;
      PROJ_STRING_LIST warnings = NULL, errors = NULL;
      PJ *object = proj_create_from_wkt(ctx, line, options, &warnings, &errors);
      printf("%.60s -> %s\n", line, object ? "obj" : "NULL");
      for (char **item = warnings; item && *item; item++) printf("  warning: %s\n", *item);
      for (char **item = errors; item && *item; item++) printf("  error: %s\n", *item);
      proj_string_list_destroy(warnings);
      proj_string_list_destroy(errors);
      proj_destroy(object);
    }
  }
  proj_context_destroy(ctx);
  return 0;
}
