/* Prints every table of proj_list_*: run against two libraries, the outputs must be identical. */
#include <stdio.h>

#include "proj.h"

int main(void) {
    for (const PJ_OPERATIONS *o = proj_list_operations(); o->id; o++)
        printf("op %s | %s\n", o->id, o->descr && *o->descr ? *o->descr : "(null)");
    for (const PJ_ELLPS *e = proj_list_ellps(); e->id; e++)
        printf("ellps %s | %s | %s | %s\n", e->id, e->major, e->ell, e->name);
    for (const PJ_UNITS *u = proj_list_units(); u->id; u++)
        printf("unit %s | %s | %s | %.17g\n", u->id, u->to_meter, u->name, u->factor);
    for (const PJ_UNITS *u = proj_list_angular_units(); u->id; u++)
        printf("angular %s | %s | %s | %.17g\n", u->id, u->to_meter, u->name, u->factor);
    for (const PJ_PRIME_MERIDIANS *p = proj_list_prime_meridians(); p->id; p++)
        printf("pm %s | %s\n", p->id, p->defn);
    return 0;
}
