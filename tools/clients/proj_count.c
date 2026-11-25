/*
 * LD_PRELOAD counter of the calls a client makes into libproj (tools/clients/gmt.sh,
 * docs/CLIENTS-gmt.md). Build: cc -shared -fPIC -O2 -I<proj include> proj_count.c -o
 * proj_count.so -ldl
 *
 * With PROJ_COUNT_LOG=<file> in the environment, every process that has a libproj mapped
 * appends one line when it starts:
 *     map <TAB> pid <TAB> exe <TAB> cwd <TAB> realpath of the libproj mapped
 * and every process that called one of the functions below appends one more when it exits:
 *     calls <TAB> pid <TAB> exe <TAB> cwd <TAB> libproj <TAB> name=calls/points ...
 * The first line is the proof of which library the process really loaded (the dynamic linker
 * already mapped every DT_NEEDED when a preloaded constructor runs); the second one counts the
 * calls of the client, not those libproj makes to itself (a call made while another counted
 * call is running on the same thread is forwarded and not counted: PROJ 9.9.0 calls its own
 * exported functions through the PLT, proj-rs does not, and the counts must compare).
 *
 * Every call is forwarded unchanged to the next definition (dlsym RTLD_NEXT). A function the
 * library does not have aborts with a message, as the dynamic linker would on a lazy binding.
 * This file is C tooling outside the crates: tests/no_unsafe only scans *.rs.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <limits.h>
#include <link.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <proj.h>

enum {
    C_CREATE,
    C_CREATE_ARGV,
    C_CREATE_CRS_TO_CRS,
    C_CREATE_CRS_TO_CRS_FROM_PJ,
    C_CREATE_FROM_WKT,
    C_CREATE_FROM_DATABASE,
    C_CREATE_OPERATIONS,
    C_TRANS,
    C_TRANS_ARRAY,
    C_TRANS_GENERIC,
    C_TRANS_BOUNDS,
    C_TRANS_BOUNDS_3D,
    C_COUNT
};

static const char *const NAMES[C_COUNT] = {
    "proj_create",
    "proj_create_argv",
    "proj_create_crs_to_crs",
    "proj_create_crs_to_crs_from_pj",
    "proj_create_from_wkt",
    "proj_create_from_database",
    "proj_create_operations",
    "proj_trans",
    "proj_trans_array",
    "proj_trans_generic",
    "proj_trans_bounds",
    "proj_trans_bounds_3D",
};

static unsigned long calls[C_COUNT];
static unsigned long points[C_COUNT];
static __thread int depth;

static void *next(int which) {
    static void *cache[C_COUNT];
    void *f = __atomic_load_n(&cache[which], __ATOMIC_ACQUIRE);
    if (f == NULL) {
        f = dlsym(RTLD_NEXT, NAMES[which]);
        if (f == NULL) {
            fprintf(stderr, "proj_count: no %s after the preload\n", NAMES[which]);
            abort();
        }
        __atomic_store_n(&cache[which], f, __ATOMIC_RELEASE);
    }
    return f;
}

static int enter(int which, unsigned long n) {
    if (depth++ == 0) {
        __atomic_fetch_add(&calls[which], 1, __ATOMIC_RELAXED);
        __atomic_fetch_add(&points[which], n, __ATOMIC_RELAXED);
        return 1;
    }
    return 0;
}

static void leave(void) { depth--; }

static int find_proj(struct dl_phdr_info *info, size_t size, void *data) {
    (void)size;
    const char *name = info->dlpi_name;
    const char *base = strrchr(name, '/');
    base = base ? base + 1 : name;
    if (strncmp(base, "libproj.so", 10) == 0 || strncmp(base, "libproj.", 8) == 0) {
        char *out = data;
        if (realpath(name, out) == NULL) {
            snprintf(out, PATH_MAX, "%s", name);
        }
        return 1;
    }
    return 0;
}

static void write_line(const char *kind, const char *tail) {
    const char *log = getenv("PROJ_COUNT_LOG");
    if (log == NULL || *log == '\0') {
        return;
    }
    char lib[PATH_MAX] = "";
    dl_iterate_phdr(find_proj, lib);
    if (lib[0] == '\0' && tail == NULL) {
        return;
    }
    char exe[PATH_MAX] = "?";
    ssize_t n = readlink("/proc/self/exe", exe, sizeof exe - 1);
    exe[n > 0 ? n : 1] = '\0';
    char cwd[PATH_MAX] = "?";
    if (getcwd(cwd, sizeof cwd) == NULL) {
        strcpy(cwd, "?");
    }
    char line[4 * PATH_MAX + 2048];
    int len = snprintf(line, sizeof line, "%s\t%d\t%s\t%s\t%s%s%s\n", kind, (int)getpid(), exe,
                       cwd, lib[0] ? lib : "-", tail ? "\t" : "", tail ? tail : "");
    if (len <= 0 || (size_t)len >= sizeof line) {
        return;
    }
    int fd = open(log, O_WRONLY | O_APPEND | O_CREAT | O_CLOEXEC, 0644);
    if (fd < 0) {
        return;
    }
    /* One write with O_APPEND: lines of concurrent processes do not interleave. */
    ssize_t w = write(fd, line, (size_t)len);
    (void)w;
    close(fd);
}

__attribute__((constructor)) static void on_load(void) { write_line("map", NULL); }

__attribute__((destructor)) static void on_exit_count(void) {
    char tail[2048] = "";
    size_t used = 0;
    for (int i = 0; i < C_COUNT; i++) {
        unsigned long c = __atomic_load_n(&calls[i], __ATOMIC_RELAXED);
        if (c == 0) {
            continue;
        }
        unsigned long p = __atomic_load_n(&points[i], __ATOMIC_RELAXED);
        int k = snprintf(tail + used, sizeof tail - used, "%s%s=%lu/%lu", used ? " " : "",
                         NAMES[i], c, p);
        if (k < 0 || (size_t)k >= sizeof tail - used) {
            break;
        }
        used += (size_t)k;
    }
    if (used > 0) {
        write_line("calls", tail);
    }
}

PJ *proj_create(PJ_CONTEXT *ctx, const char *definition) {
    PJ *(*f)(PJ_CONTEXT *, const char *) = next(C_CREATE);
    enter(C_CREATE, 0);
    PJ *r = f(ctx, definition);
    leave();
    return r;
}

PJ *proj_create_argv(PJ_CONTEXT *ctx, int argc, char **argv) {
    PJ *(*f)(PJ_CONTEXT *, int, char **) = next(C_CREATE_ARGV);
    enter(C_CREATE_ARGV, 0);
    PJ *r = f(ctx, argc, argv);
    leave();
    return r;
}

PJ *proj_create_crs_to_crs(PJ_CONTEXT *ctx, const char *source_crs, const char *target_crs,
                           PJ_AREA *area) {
    PJ *(*f)(PJ_CONTEXT *, const char *, const char *, PJ_AREA *) = next(C_CREATE_CRS_TO_CRS);
    enter(C_CREATE_CRS_TO_CRS, 0);
    PJ *r = f(ctx, source_crs, target_crs, area);
    leave();
    return r;
}

PJ *proj_create_crs_to_crs_from_pj(PJ_CONTEXT *ctx, const PJ *source_crs, const PJ *target_crs,
                                   PJ_AREA *area, const char *const *options) {
    PJ *(*f)(PJ_CONTEXT *, const PJ *, const PJ *, PJ_AREA *, const char *const *) =
        next(C_CREATE_CRS_TO_CRS_FROM_PJ);
    enter(C_CREATE_CRS_TO_CRS_FROM_PJ, 0);
    PJ *r = f(ctx, source_crs, target_crs, area, options);
    leave();
    return r;
}

PJ *proj_create_from_wkt(PJ_CONTEXT *ctx, const char *wkt, const char *const *options,
                         PROJ_STRING_LIST *out_warnings, PROJ_STRING_LIST *out_grammar_errors) {
    PJ *(*f)(PJ_CONTEXT *, const char *, const char *const *, PROJ_STRING_LIST *,
             PROJ_STRING_LIST *) = next(C_CREATE_FROM_WKT);
    enter(C_CREATE_FROM_WKT, 0);
    PJ *r = f(ctx, wkt, options, out_warnings, out_grammar_errors);
    leave();
    return r;
}

PJ *proj_create_from_database(PJ_CONTEXT *ctx, const char *auth_name, const char *code,
                              PJ_CATEGORY category, int usePROJAlternativeGridNames,
                              const char *const *options) {
    PJ *(*f)(PJ_CONTEXT *, const char *, const char *, PJ_CATEGORY, int, const char *const *) =
        next(C_CREATE_FROM_DATABASE);
    enter(C_CREATE_FROM_DATABASE, 0);
    PJ *r = f(ctx, auth_name, code, category, usePROJAlternativeGridNames, options);
    leave();
    return r;
}

PJ_OBJ_LIST *proj_create_operations(PJ_CONTEXT *ctx, const PJ *source_crs, const PJ *target_crs,
                                    const PJ_OPERATION_FACTORY_CONTEXT *operationContext) {
    PJ_OBJ_LIST *(*f)(PJ_CONTEXT *, const PJ *, const PJ *,
                      const PJ_OPERATION_FACTORY_CONTEXT *) = next(C_CREATE_OPERATIONS);
    enter(C_CREATE_OPERATIONS, 0);
    PJ_OBJ_LIST *r = f(ctx, source_crs, target_crs, operationContext);
    leave();
    return r;
}

PJ_COORD proj_trans(PJ *P, PJ_DIRECTION direction, PJ_COORD coord) {
    PJ_COORD (*f)(PJ *, PJ_DIRECTION, PJ_COORD) = next(C_TRANS);
    enter(C_TRANS, 1);
    PJ_COORD r = f(P, direction, coord);
    leave();
    return r;
}

int proj_trans_array(PJ *P, PJ_DIRECTION direction, size_t n, PJ_COORD *coord) {
    int (*f)(PJ *, PJ_DIRECTION, size_t, PJ_COORD *) = next(C_TRANS_ARRAY);
    enter(C_TRANS_ARRAY, n);
    int r = f(P, direction, n, coord);
    leave();
    return r;
}

size_t proj_trans_generic(PJ *P, PJ_DIRECTION direction, double *x, size_t sx, size_t nx,
                          double *y, size_t sy, size_t ny, double *z, size_t sz, size_t nz,
                          double *t, size_t st, size_t nt) {
    size_t (*f)(PJ *, PJ_DIRECTION, double *, size_t, size_t, double *, size_t, size_t,
                double *, size_t, size_t, double *, size_t, size_t) = next(C_TRANS_GENERIC);
    size_t n = nx;
    if (ny > n) n = ny;
    if (nz > n) n = nz;
    if (nt > n) n = nt;
    enter(C_TRANS_GENERIC, n);
    size_t r = f(P, direction, x, sx, nx, y, sy, ny, z, sz, nz, t, st, nt);
    leave();
    return r;
}

int proj_trans_bounds(PJ_CONTEXT *context, PJ *P, PJ_DIRECTION direction, double xmin,
                      double ymin, double xmax, double ymax, double *out_xmin, double *out_ymin,
                      double *out_xmax, double *out_ymax, int densify_pts) {
    int (*f)(PJ_CONTEXT *, PJ *, PJ_DIRECTION, double, double, double, double, double *,
             double *, double *, double *, int) = next(C_TRANS_BOUNDS);
    enter(C_TRANS_BOUNDS, 1);
    int r = f(context, P, direction, xmin, ymin, xmax, ymax, out_xmin, out_ymin, out_xmax,
              out_ymax, densify_pts);
    leave();
    return r;
}

int proj_trans_bounds_3D(PJ_CONTEXT *context, PJ *P, PJ_DIRECTION direction, double xmin,
                         double ymin, double zmin, double xmax, double ymax, double zmax,
                         double *out_xmin, double *out_ymin, double *out_zmin, double *out_xmax,
                         double *out_ymax, double *out_zmax, int densify_pts) {
    int (*f)(PJ_CONTEXT *, PJ *, PJ_DIRECTION, double, double, double, double, double, double,
             double *, double *, double *, double *, double *, double *, int) =
        next(C_TRANS_BOUNDS_3D);
    enter(C_TRANS_BOUNDS_3D, 1);
    int r = f(context, P, direction, xmin, ymin, zmin, xmax, ymax, zmax, out_xmin, out_ymin,
              out_zmin, out_xmax, out_ymax, out_zmax, densify_pts);
    leave();
    return r;
}
