/*
 * The libproj side of `proj-rs-bench`, as a separate program: the bench calls libproj without
 * any `unsafe` in Rust (CLAUDE.md), by sending the points here and reading the times back.
 * build.rs compiles it against the libproj it finds (PROJ_RS_LIBPROJ_DIR, else CONDA_PREFIX).
 *
 * The timing is the bench's, transcribed from src/bin/bench.rs and bench_billion.rs: each case
 * is the median of 3 trials, each trial one untimed run then the median of 11 timed runs; the
 * billion case is one wall-clock run over the whole billion. The points are those the bench
 * computed, bit for bit.
 *
 * Protocol, on stdin: one command line, then the points as native-endian doubles (x, y pairs).
 *   pipeline <n> <check_utm 0|1> <definition>          -> "ok <ns>"
 *   facade <n> <from> <to>                             -> "ok <ns_point> <ns_array>" + n pairs
 *   billion <n> <total> <from> <to>                    -> "ok <s_point> <s_array>" + n pairs
 * The pairs written back are the images of the slice call (the first pass for `billion`), which
 * the bench compares with proj-rs. Any failure prints "err <message>" and exits with 1.
 */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#include <windows.h>
#else
#include <time.h>
#endif

typedef struct {
    double x, y, z, t;
} PjCoord;

void *proj_context_create(void);
void proj_context_destroy(void *ctx);
void *proj_create(void *ctx, const char *definition);
void *proj_destroy(void *pj);
PjCoord proj_trans(void *pj, int direction, PjCoord coord);
int proj_log_level(void *ctx, int level);
void *proj_create_crs_to_crs(void *ctx, const char *source, const char *target, void *area);
void *proj_normalize_for_visualization(void *ctx, const void *obj);
size_t proj_trans_generic(void *pj, int direction, double *x, size_t sx, size_t nx, double *y,
                          size_t sy, size_t ny, double *z, size_t sz, size_t nz, double *t,
                          size_t st, size_t nt);

#define TRIALS 3
#define REPEATS 11

static volatile double sink;

static double now_seconds(void) {
#ifdef _WIN32
    LARGE_INTEGER count, frequency;
    QueryPerformanceCounter(&count);
    QueryPerformanceFrequency(&frequency);
    return (double)count.QuadPart / (double)frequency.QuadPart;
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
#endif
}

static void fail(const char *message) {
    printf("err %s\n", message);
    fflush(stdout);
    exit(1);
}

static int compare(const void *a, const void *b) {
    double x = *(const double *)a, y = *(const double *)b;
    return (x > y) - (x < y);
}

static double median(double *values, size_t n) {
    qsort(values, n, sizeof values[0], compare);
    return values[n / 2];
}

/* A case of the bench: what one run does. */
typedef struct {
    void *pj;
    const double *points;  /* n interleaved (x, y) pairs */
    const PjCoord *coords; /* the same points as `PJ_COORD`, built before the timing */
    double *buffer;        /* n pairs, for the slice call */
    size_t n;
} Case;

static void run_points(const Case *c) {
    double sum = 0.0;
    for (size_t i = 0; i < c->n; i++) {
        sum += proj_trans(c->pj, 1, c->coords[i]).x;
    }
    sink = sum;
}

/* The points as `PJ_COORD`, as bench.rs built its `Vec<PjCoord>`: t = 0 for a pipeline, t = +inf
 * for the facade. */
static PjCoord *coords_of(const double *points, size_t n, double t) {
    PjCoord *coords = malloc(n * sizeof(PjCoord));
    if (coords == NULL) {
        fail("allocation");
    }
    for (size_t i = 0; i < n; i++) {
        PjCoord coord = {points[2 * i], points[2 * i + 1], 0.0, t};
        coords[i] = coord;
    }
    return coords;
}

static void run_array(const Case *c) {
    memcpy(c->buffer, c->points, c->n * 2 * sizeof(double));
    sink = (double)proj_trans_generic(c->pj, 1, c->buffer, 16, c->n, c->buffer + 1, 16, c->n,
                                      NULL, 0, 0, NULL, 0, 0);
}

/* `time_stable` of bench.rs: nanoseconds per point. */
static double time_stable(const Case *c, void (*run)(const Case *)) {
    double trials[TRIALS];
    for (int trial = 0; trial < TRIALS; trial++) {
        double samples[REPEATS];
        run(c);
        for (int sample = 0; sample < REPEATS; sample++) {
            double start = now_seconds();
            run(c);
            samples[sample] = (now_seconds() - start) * 1e9 / (double)c->n;
        }
        trials[trial] = median(samples, REPEATS);
    }
    return median(trials, TRIALS);
}

static double *read_points(size_t n) {
    double *points = malloc(n * 2 * sizeof(double));
    if (points == NULL || fread(points, sizeof(double), n * 2, stdin) != n * 2) {
        fail("short read of the points");
    }
    return points;
}

static void write_points(const double *points, size_t n) {
    fwrite(points, sizeof(double), n * 2, stdout);
}

static void *crs_to_crs(void *ctx, const char *from, const char *to) {
    void *raw = proj_create_crs_to_crs(ctx, from, to, NULL);
    if (raw == NULL) {
        fail("proj_create_crs_to_crs");
    }
    void *pj = proj_normalize_for_visualization(ctx, raw);
    if (pj == NULL) {
        fail("proj_normalize_for_visualization");
    }
    proj_destroy(raw);
    return pj;
}

/* `refill` of bench_billion.rs: the source shifted east by a micro-degree per pass. */
static void refill(double *buffer, const double *source, size_t n, size_t pass) {
    double shift = (double)pass * 1e-6;
    for (size_t i = 0; i < n; i++) {
        buffer[2 * i] = source[2 * i] + shift;
        buffer[2 * i + 1] = source[2 * i + 1];
    }
}

static void billion(void *pj, size_t n, size_t total, const double *source) {
    double *buffer = malloc(n * 2 * sizeof(double));
    double *first = NULL;
    if (buffer == NULL) {
        fail("allocation");
    }
    memcpy(buffer, source, n * 2 * sizeof(double));
    double start = now_seconds();
    for (size_t pass = 0; pass < total / n; pass++) {
        refill(buffer, source, n, pass);
        for (size_t i = 0; i < n; i++) {
            PjCoord coord = {buffer[2 * i], buffer[2 * i + 1], 0.0, HUGE_VAL};
            PjCoord out = proj_trans(pj, 1, coord);
            buffer[2 * i] = out.x;
            buffer[2 * i + 1] = out.y;
        }
        sink = buffer[0];
    }
    double one_by_one = now_seconds() - start;
    memcpy(buffer, source, n * 2 * sizeof(double));
    start = now_seconds();
    for (size_t pass = 0; pass < total / n; pass++) {
        refill(buffer, source, n, pass);
        sink = (double)proj_trans_generic(pj, 1, buffer, 16, n, buffer + 1, 16, n, NULL, 0, 0,
                                          NULL, 0, 0);
        if (first == NULL) {
            first = malloc(n * 2 * sizeof(double));
            if (first == NULL) {
                fail("allocation");
            }
            memcpy(first, buffer, n * 2 * sizeof(double));
        }
        sink = buffer[0];
    }
    double array = now_seconds() - start;
    printf("ok %.17g %.17g\n", one_by_one, array);
    write_points(first, n);
    free(first);
    free(buffer);
}

int main(void) {
    char line[4096];
#ifdef _WIN32
    _setmode(_fileno(stdin), _O_BINARY);
    _setmode(_fileno(stdout), _O_BINARY);
#endif
    void *ctx = proj_context_create();
    if (ctx == NULL) {
        fail("proj_context_create");
    }
    while (fgets(line, sizeof line, stdin) != NULL) {
        line[strcspn(line, "\n")] = '\0';
        char from[512], to[512];
        size_t n = 0, total = 0;
        int check = 0, used = 0;
        if (sscanf(line, "pipeline %zu %d %n", &n, &check, &used) == 2 && used > 0) {
            const char *definition = line + used;
            double *points = read_points(n);
            void *pj = proj_create(ctx, definition);
            if (pj == NULL) {
                fail("proj_create");
            }
            if (check) {
                PjCoord probe = {3.0 * 3.14159265358979323846 / 180.0, 0.0, 0.0, 0.0};
                PjCoord out = proj_trans(pj, 1, probe);
                if (fabs(out.x - 500000.0) >= 1.0) {
                    fail("libproj UTM easting, ABI or definition is wrong");
                }
            }
            PjCoord *coords = coords_of(points, n, 0.0);
            Case c = {pj, points, coords, NULL, n};
            printf("ok %.17g\n", time_stable(&c, run_points));
            proj_destroy(pj);
            free(coords);
            free(points);
        } else if (sscanf(line, "facade %zu %511s %511s", &n, from, to) == 3) {
            double *points = read_points(n);
            double *buffer = malloc(n * 2 * sizeof(double));
            if (buffer == NULL) {
                fail("allocation");
            }
            /* PJ_LOG_NONE: libproj reports each missing grid of the NAD27 candidates on
             * stderr, which says nothing about the timing. */
            proj_log_level(ctx, 0);
            void *pj = crs_to_crs(ctx, from, to);
            PjCoord *coords = coords_of(points, n, HUGE_VAL);
            Case c = {pj, points, coords, buffer, n};
            double one_by_one = time_stable(&c, run_points);
            double array = time_stable(&c, run_array);
            printf("ok %.17g %.17g\n", one_by_one, array);
            write_points(buffer, n);
            proj_destroy(pj);
            free(coords);
            free(buffer);
            free(points);
        } else if (sscanf(line, "billion %zu %zu %511s %511s", &n, &total, from, to) == 4) {
            double *points = read_points(n);
            proj_log_level(ctx, 0);
            void *pj = crs_to_crs(ctx, from, to);
            billion(pj, n, total, points);
            proj_destroy(pj);
            free(points);
        } else {
            fail("unknown command");
        }
        fflush(stdout);
    }
    proj_context_destroy(ctx);
    return 0;
}
