# Compares the stdout of two cs2cs runs line by line. Inputs, as -v:
# refFile, portFile, tol, rcr, rcp, pair. Prints one TSV row: pair, status,
# max |delta| per field, reference line, our line, exit codes ref/port.

function isnum(s) { return s ~ /^[-+]?([0-9]+[.]?[0-9]*|[.][0-9]+)([eE][-+]?[0-9]+)?$/ }
function bump(s) { if (rank[s] > rank[worst]) worst = s }
function clean(s) { gsub(/\t/, " ", s); return s }

BEGIN {
    rank["same"] = 0; rank["tol"] = 1; rank["text"] = 2
    rank["num"] = 3; rank["lines"] = 4; rank["status"] = 5
    worst = "same"
    while ((getline line < refFile) > 0) R[++nr] = line
    while ((getline line < portFile) > 0) P[++np] = line
    if (rcr != rcp) bump("status")
    if (nr != np) bump("lines")
    first = 0
    nmax = 0
    n = (nr > np) ? nr : np
    for (i = 1; i <= n; i++) {
        if (i > nr || i > np) { if (!first) first = i; continue }
        if (R[i] == P[i]) continue
        if (!first) first = i
        na = split(R[i], a)
        nb = split(P[i], b)
        ea = (a[1] == "*")
        eb = (b[1] == "*")
        if (ea || eb) { if (ea != eb) bump("status"); else bump("text"); continue }
        if (na != nb) { bump("text"); continue }
        for (k = 1; k <= na; k++) {
            if (a[k] == b[k]) continue
            if (!isnum(a[k]) || !isnum(b[k])) { bump("text"); continue }
            d = a[k] - b[k]
            if (d < 0) d = -d
            if (d > mx[k]) mx[k] = d
            if (k > nmax) nmax = k
            if (d > tol) bump("num"); else bump("tol")
        }
    }
    if (!first) first = 1
    dstr = ""
    for (k = 1; k <= nmax; k++) dstr = dstr (k > 1 ? ";" : "") "f" k "=" sprintf("%.3g", mx[k] + 0)
    if (dstr == "") dstr = "-"
    refl = (first <= nr) ? clean(R[first]) : "-"
    portl = (first <= np) ? clean(P[first]) : "-"
    printf "%s\t%s\t%s\t%s\t%s\t%s/%s\n", clean(pair), worst, dstr, refl, portl, rcr, rcp
}
