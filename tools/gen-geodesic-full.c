/* Reference values of tests/geodesic_full.rs: what `geodesic.c` of PROJ 9.9.0 computes.
 *
 *   tools/build-reference.sh
 *   cc -O0 -ffp-contract=off -I target/proj-reference/src/src \
 *      tools/gen-geodesic-full.c target/proj-reference/src/src/geodesic.c -lm -o /tmp/gen-geodesic-full
 *   /tmp/gen-geodesic-full > tests/geodesic/full.txt
 *
 * One line per call, the inputs then `|` then the outputs, in %.17g. The deterministic generator
 * below is the only source of inputs. Kinds: I (geod_geninverse), D (geod_gendirect),
 * L (geod_inverseline, then geod_genposition along it), P (geod_polygon_*).
 */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include "geodesic.h"

static unsigned long long state = 0x9E3779B97F4A7C15ULL;
static double rnd(void) {
  state ^= state << 13; state ^= state >> 7; state ^= state << 17;
  return (double)(state >> 11) / 9007199254740992.0;
}
static double uni(double lo, double hi) { return lo + (hi - lo) * rnd(); }

struct ell { const char *name; double a, f; };
static const struct ell ELL[] = {
  {"wgs84", 6378137.0, 1 / 298.257223563},
  {"grs80", 6378137.0, 1 / 298.257222101},
  {"bessel", 6377397.155, 1 / 299.1528128},
  {"sphere", 6371000.0, 0.0},
  {"flat20", 6378137.0, 0.05},
  {"prolate", 6378137.0, -0.003352810664747480},
  {"unit", 1.0, 1 / 297.0},
};
#define NELL (sizeof ELL / sizeof ELL[0])

static void inverse(const struct ell *e, const struct geod_geodesic *g,
                    double la1, double lo1, double la2, double lo2) {
  double s12, azi1, azi2, m12, M12, M21, S12;
  double a12 = geod_geninverse(g, la1, lo1, la2, lo2, &s12, &azi1, &azi2,
                               &m12, &M12, &M21, &S12);
  printf("I %s %.17g %.17g %.17g %.17g | %.17g %.17g %.17g %.17g %.17g %.17g %.17g %.17g\n",
         e->name, la1, lo1, la2, lo2, a12, s12, azi1, azi2, m12, M12, M21, S12);
}

static void direct(const struct ell *e, const struct geod_geodesic *g,
                   double la1, double lo1, double azi1, unsigned flags, double s) {
  double lat2, lon2, azi2, s12, m12, M12, M21, S12;
  double a12 = geod_gendirect(g, la1, lo1, azi1, flags, s, &lat2, &lon2, &azi2,
                              &s12, &m12, &M12, &M21, &S12);
  printf("D %s %.17g %.17g %.17g %u %.17g | %.17g %.17g %.17g %.17g %.17g %.17g %.17g %.17g %.17g\n",
         e->name, la1, lo1, azi1, flags, s, a12, lat2, lon2, azi2, s12, m12, M12, M21, S12);
}

static void line(const struct ell *e, const struct geod_geodesic *g,
                 double la1, double lo1, double la2, double lo2) {
  struct geod_geodesicline l;
  unsigned caps = GEOD_LATITUDE | GEOD_LONGITUDE | GEOD_AZIMUTH | GEOD_DISTANCE |
                  GEOD_REDUCEDLENGTH | GEOD_GEODESICSCALE | GEOD_AREA | GEOD_DISTANCE_IN;
  int i;
  geod_inverseline(&l, g, la1, lo1, la2, lo2, caps);
  printf("L %s %.17g %.17g %.17g %.17g | %.17g %.17g %.17g", e->name, la1, lo1, la2, lo2,
         l.azi1, l.a13, l.s13);
  for (i = 1; i <= 3; ++i) {
    double lat, lon, azi, s12, m12, M12, M21, S12;
    double a12 = geod_genposition(&l, 0, l.s13 * i / 4, &lat, &lon, &azi, &s12, &m12,
                                  &M12, &M21, &S12);
    printf(" %.17g %.17g %.17g %.17g %.17g %.17g %.17g %.17g %.17g", a12, lat, lon, azi, s12, m12,
           M12, M21, S12);
  }
  printf("\n");
}

#define MAXN 12
static void polygon(const struct ell *e, const struct geod_geodesic *g, int n,
                    const double *lat, const double *lon) {
  struct geod_polygon p, q;
  double A, P, extra_lat = uni(-80, 80), extra_lon = uni(-180, 180);
  double eazi = uni(-180, 180), edist = uni(1e3, 3e6);
  int i, rev, sgn;
  printf("P %s %d", e->name, n);
  for (i = 0; i < n; ++i) printf(" %.17g %.17g", lat[i], lon[i]);
  printf(" %.17g %.17g %.17g %.17g |", extra_lat, extra_lon, eazi, edist);
  geod_polygon_init(&p, 0);
  for (i = 0; i < n; ++i) geod_polygon_addpoint(g, &p, lat[i], lon[i]);
  for (rev = 0; rev < 2; ++rev)
    for (sgn = 0; sgn < 2; ++sgn) {
      geod_polygon_compute(g, &p, rev, sgn, &A, &P);
      printf(" %.17g %.17g", A, P);
    }
  geod_polygon_testpoint(g, &p, extra_lat, extra_lon, 0, 1, &A, &P);
  printf(" %.17g %.17g", A, P);
  geod_polygon_testedge(g, &p, eazi, edist, 1, 0, &A, &P);
  printf(" %.17g %.17g", A, P);
  geod_polygon_addedge(g, &p, eazi, edist);
  geod_polygon_compute(g, &p, 0, 1, &A, &P);
  printf(" %.17g %.17g", A, P);
  geod_polygon_init(&q, 1);
  for (i = 0; i < n; ++i) geod_polygon_addpoint(g, &q, lat[i], lon[i]);
  geod_polygon_compute(g, &q, 0, 1, NULL, &P);
  printf(" %.17g", P);
  {
    double lats[MAXN], lons[MAXN];
    for (i = 0; i < n; ++i) { lats[i] = lat[i]; lons[i] = lon[i]; }
    geod_polygonarea(g, lats, lons, n, &A, &P);
    printf(" %.17g %.17g\n", A, P);
  }
}

int main(void) {
  static const double HARD[] = {0, 1e-9, 0.5, 30, 45, 60, 89.9999, 90, -90, -45, 179.5, 180, -180, 179.99999};
  const int NH = sizeof HARD / sizeof HARD[0];
  size_t k;
  int i, j, m;
  for (k = 0; k < NELL; ++k) {
    struct geod_geodesic g;
    geod_init(&g, ELL[k].a, ELL[k].f);
    /* Inverse: the hard grid, then random pairs, at every ellipsoid. */
    for (i = 0; i < NH; ++i)
      for (j = 0; j < NH && (k == 0 || k == 3 || k == 5); ++j) {
        double la1 = HARD[i] > 90 || HARD[i] < -90 ? 10 : HARD[i];
        double la2 = HARD[j] > 90 || HARD[j] < -90 ? -10 : HARD[j];
        inverse(&ELL[k], &g, la1, 0, la2, HARD[(i + j) % NH]);
        inverse(&ELL[k], &g, la1, 20, la2, 20 + HARD[j]);
      }
    for (m = 0; m < 10; ++m) {
      double la1 = uni(-90, 90), lo1 = uni(-180, 180), la2 = uni(-90, 90), lo2 = uni(-180, 180);
      inverse(&ELL[k], &g, la1, lo1, la2, lo2);
      /* a short line and a nearly antipodal one */
      inverse(&ELL[k], &g, la1, lo1, la1 + uni(-1, 1) * 1e-3, lo1 + uni(-1, 1) * 1e-3);
      inverse(&ELL[k], &g, la1, lo1, -la1 + uni(-1, 1) * 1e-2, lo1 + 180 + uni(-1, 1) * 1e-2);
      line(&ELL[k], &g, la1, lo1, la2, lo2);
    }
    /* Direct: distance, arc and unrolled longitude. */
    for (m = 0; m < 10; ++m) {
      double la1 = uni(-90, 90), lo1 = uni(-180, 180), azi = uni(-180, 180);
      double s = uni(0, 2e7) * ELL[k].a / 6378137.0;
      direct(&ELL[k], &g, la1, lo1, azi, 0, s);
      direct(&ELL[k], &g, la1, lo1, azi, GEOD_ARCMODE, uni(0, 540));
      direct(&ELL[k], &g, la1, lo1, azi, GEOD_LONG_UNROLL, 3 * s);
      direct(&ELL[k], &g, la1, lo1, azi, GEOD_ARCMODE | GEOD_LONG_UNROLL, uni(-720, 720));
    }
    for (i = 0; i < NH && (k == 0 || k == 3 || k == 5); ++i) {
      double la1 = HARD[i] > 90 || HARD[i] < -90 ? 10 : HARD[i];
      direct(&ELL[k], &g, la1, 0, 0, 0, 1e6 * ELL[k].a / 6378137.0);
      direct(&ELL[k], &g, la1, 0, 90, 0, 1e7 * ELL[k].a / 6378137.0);
      direct(&ELL[k], &g, la1, 0, 180, GEOD_LONG_UNROLL, 3e7 * ELL[k].a / 6378137.0);
      direct(&ELL[k], &g, la1, 170, 45, GEOD_LONG_UNROLL, 4e7 * ELL[k].a / 6378137.0);
    }
    /* Polygons: random ones, ones that cross the prime meridian and the antimeridian, and one
     * around a pole. */
    for (m = 0; m < 8; ++m) {
      double lat[MAXN], lon[MAXN];
      int n = 3 + (int)(rnd() * 8);
      double clat = uni(-80, 80), clon = uni(-180, 180), r = uni(0.01, 40);
      for (i = 0; i < n; ++i) {
        double t = 2 * M_PI * i / n;
        lat[i] = clat + r * sin(t) * 0.5;
        lon[i] = clon + r * cos(t);
        if (lat[i] > 89.9) lat[i] = 89.9;
        if (lat[i] < -89.9) lat[i] = -89.9;
      }
      polygon(&ELL[k], &g, n, lat, lon);
    }
    {
      double lat[4] = {85, 85, 85, 85}, lon[4] = {0, 90, 180, -90};
      double plat[3] = {0, 0, 45}, plon[3] = {-1, 1, 0};
      double alat[4] = {10, 10, -10, -10}, alon[4] = {179, -179, -179, 179};
      polygon(&ELL[k], &g, 4, lat, lon);
      polygon(&ELL[k], &g, 3, plat, plon);
      polygon(&ELL[k], &g, 4, alat, alon);
    }
  }
  return 0;
}
