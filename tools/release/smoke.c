/* Smoke test of the C library archive: it is compiled against include/ and lib/ of the archive
   and run. Prints one line per check; tools/release-smoke.{sh,ps1} compare the output. */
#include <proj.h>
#include <stdio.h>

int main(void) {
    PJ_INFO info = proj_info();
    printf("version %d.%d.%d\n", info.major, info.minor, info.patch);

    PJ_CONTEXT *ctx = proj_context_create();
    PJ *crs_to_crs = proj_create_crs_to_crs(ctx, "EPSG:4326", "EPSG:2154", NULL);
    if (crs_to_crs == NULL) {
        printf("proj_create_crs_to_crs failed: %s\n", proj_errno_string(proj_context_errno(ctx)));
        return 1;
    }
    /* EPSG:4326 is latitude, longitude. */
    PJ_COORD paris = proj_trans(crs_to_crs, PJ_FWD, proj_coord(48.8584, 2.2945, 0, 0));
    printf("lambert93 %.3f %.3f\n", paris.xy.x, paris.xy.y);
    PJ_COORD back = proj_trans(crs_to_crs, PJ_INV, paris);
    printf("inverse %.6f %.6f\n", back.xy.x, back.xy.y);
    proj_destroy(crs_to_crs);

    PJ *utm = proj_create(ctx, "+proj=utm +zone=31 +ellps=WGS84");
    if (utm == NULL) {
        printf("proj_create failed\n");
        return 1;
    }
    PJ_COORD east = proj_trans(utm, PJ_FWD, proj_coord(proj_torad(2.0), proj_torad(49.0), 0, 0));
    printf("utm31 %.3f %.3f\n", east.xy.x, east.xy.y);
    proj_destroy(utm);
    proj_context_destroy(ctx);
    return 0;
}
