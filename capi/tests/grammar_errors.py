#!/usr/bin/env python3
"""Compare the WKT grammar errors of two `wkt_warnings check` outputs.

Usage: grammar_errors.py <input> <libproj output> <proj-rs output>

A record is the `<first 60 bytes> -> obj|NULL` line of an input line and the lines that
follow it. A "Parsing error" (pj_wkt1_parse / pj_wkt2_parse) spans three lines. Records whose
object and other warnings and errors agree must have identical grammar errors; records where
they differ are counted apart, as builder differences. Exit status 1 on a grammar difference.
"""

import sys


def records(inputs, path):
    with open(path, "rb") as handle:
        data = handle.read().split(b"\n")
    out = []
    k = 0
    for i, line in enumerate(inputs):
        header = line[:60] + b" -> "
        if k >= len(data) or not data[k].startswith(header):
            sys.exit("%s: no record for input line %d" % (path, i + 1))
        record = (data[k], [], [])
        k += 1
        following = inputs[i + 1][:60] + b" -> " if i + 1 < len(inputs) else None
        while k < len(data):
            if following is not None and data[k] in (following + b"obj", following + b"NULL"):
                break
            if data[k].startswith(b"  error: Parsing error"):
                record[1].append(b"\n".join(data[k : k + 3]))
                k += 3
            else:
                if data[k]:
                    record[2].append(data[k])
                k += 1
        out.append(record)
    return out


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    with open(sys.argv[1], "rb") as handle:
        inputs = handle.read().split(b"\n")
    if inputs and inputs[-1] == b"":
        inputs.pop()
    theirs = records(inputs, sys.argv[2])
    ours = records(inputs, sys.argv[3])
    compared = builder = grammar = with_errors = 0
    for line, ref, got in zip(inputs, theirs, ours):
        if ref[0] != got[0] or ref[2] != got[2]:
            builder += 1
            continue
        compared += 1
        with_errors += 1 if ref[1] else 0
        if ref[1] != got[1]:
            grammar += 1
            if grammar <= 10:
                sys.stderr.write("input: %s\n" % line[:200].decode("utf-8", "replace"))
                for name, errors in (("libproj", ref[1]), ("proj-rs", got[1])):
                    for error in errors:
                        sys.stderr.write("%s:\n%s\n" % (name, error.decode("utf-8", "replace")))
    print(
        "%d malformed WKT: %d compared (%d with a grammar error), %d with other differences, "
        "%d grammar differences" % (len(inputs), compared, with_errors, builder, grammar)
    )
    sys.exit(1 if grammar else 0)


if __name__ == "__main__":
    main()
