/* A WKT or PROJ string handed over in Latin-1, as QGIS does (QString::toLatin1): a degree sign is
   the single byte 0xB0, which is not UTF-8. PROJ reads the bytes. For each case: the object of
   proj_create_from_wkt and of proj_create (null or not, type, identifier) and the dialect
   proj_context_guess_wkt_dialect gives. run.sh compares the output with libproj of PROJ 9.9.0. */
#include <proj.h>
#include <stdio.h>

static void show(const char *who, PJ *object) {
    if (!object) {
        printf("  %s null\n", who);
        return;
    }
    const char *auth = proj_get_id_auth_name(object, 0), *code = proj_get_id_code(object, 0);
    printf("  %s type=%d id=%s:%s\n", who, (int)proj_get_type(object), auth ? auth : "-",
           code ? code : "-");
    proj_destroy(object);
}

int main(void) {
    const char *cases[] = {
        "PROJCRS[\"ETRS89 / UTM zone 32N\",BASEGEOGCRS[\"ETRS89\",DATUM[\"European Terrestrial "
        "Reference System 1989\",ELLIPSOID[\"GRS 1980\",6378137,298.257222101]]],CONVERSION[\"UTM "
        "zone 32N\",METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]],PARAMETER[\"Latitude of "
        "natural origin\",0,ANGLEUNIT[\"degree\",0.0174532925199433]],PARAMETER[\"Longitude of "
        "natural origin\",9,ANGLEUNIT[\"degree\",0.0174532925199433]],PARAMETER[\"Scale factor at "
        "natural origin\",0.9996,SCALEUNIT[\"unity\",1]],PARAMETER[\"False easting\",500000,"
        "LENGTHUNIT[\"metre\",1]],PARAMETER[\"False northing\",0,LENGTHUNIT[\"metre\",1]]],"
        "CS[Cartesian,2],AXIS[\"easting\",east,ORDER[1],LENGTHUNIT[\"metre\",1]],AXIS[\"northing\","
        "north,ORDER[2],LENGTHUNIT[\"metre\",1]],USAGE[SCOPE[\"Engineering survey\"],AREA[\"Europe "
        "between 6\xb0" "E and 12\xb0" "E\"],BBOX[38.76,6,84.33,12]],ID[\"EPSG\",25832]]",
        "GEOGCS[\"WGS 84 \xb0\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563]],"
        "PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433],AUTHORITY[\"EPSG\",\"4326\"]]",
    };
    for (unsigned i = 0; i < sizeof cases / sizeof cases[0]; i++) {
        PROJ_STRING_LIST warnings = NULL, errors = NULL;
        printf("case %u dialect=%d\n", i, (int)proj_context_guess_wkt_dialect(NULL, cases[i]));
        show("from_wkt", proj_create_from_wkt(NULL, cases[i], NULL, &warnings, &errors));
        proj_string_list_destroy(warnings);
        proj_string_list_destroy(errors);
        show("create", proj_create(NULL, cases[i]));
    }
    return 0;
}
