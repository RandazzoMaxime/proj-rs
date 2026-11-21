/* proj_create of objects that are not CRS, as createFromUserInput reads them (io.cpp 7652-7707
   for the URN, the WKT and PROJJSON of any object): for each argument, type, name and
   identifier. run.sh compares the output with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>
int main(int argc, char **argv) {
    for (int i = 1; i < argc; i++) {
        PJ *o = proj_create(NULL, argv[i]);
        const char *n = o ? proj_get_name(o) : NULL, *a = o ? proj_get_id_auth_name(o, 0) : NULL,
                   *c = o ? proj_get_id_code(o, 0) : NULL;
        printf("%.50s\n  %s type=%d name=%s id=%s:%s\n", argv[i], o ? "ok" : "null",
               o ? (int)proj_get_type(o) : -1, n ? n : "-", a ? a : "-", c ? c : "-");
        if (o && proj_get_type(o) >= PJ_TYPE_CONVERSION && proj_get_type(o) <= PJ_TYPE_OTHER_COORDINATE_OPERATION) {
            PJ_PROJ_INFO info = proj_pj_info(o);
            printf("  info: %s | %g\n", info.description ? info.description : "-", info.accuracy);
        }
        proj_destroy(o);
    }
    return 0;
}
