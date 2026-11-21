/* The layout of the structures of geodesic.h, which the C caller allocates: the sizes and the
   offsets that capi/src/geodesic.rs and src/geodesic_c.rs rely on. Compiles, or the layout moved. */
#include <stddef.h>
#include "geodesic.h"

#define STATIC_ASSERT(condition, message) typedef char message[(condition) ? 1 : -1]

STATIC_ASSERT(sizeof(struct geod_geodesic) == 51 * 8, size_of_geod_geodesic);
STATIC_ASSERT(offsetof(struct geod_geodesic, C4x) == 30 * 8, offset_of_geod_geodesic_C4x);
STATIC_ASSERT(sizeof(struct geod_geodesicline) == 64 * 8, size_of_geod_geodesicline);
STATIC_ASSERT(offsetof(struct geod_geodesicline, B41) == 29 * 8, offset_of_geod_geodesicline_B41);
STATIC_ASSERT(offsetof(struct geod_geodesicline, caps) == 63 * 8, offset_of_geod_geodesicline_caps);
STATIC_ASSERT(sizeof(struct geod_polygon) == 10 * 8, size_of_geod_polygon);
STATIC_ASSERT(offsetof(struct geod_polygon, polyline) == 8 * 8, offset_of_geod_polygon_polyline);
STATIC_ASSERT(offsetof(struct geod_polygon, num) == 8 * 8 + 8, offset_of_geod_polygon_num);
