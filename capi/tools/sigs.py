"""Shared reader of capi/tools/proj_h_functions.rs.txt: name -> list of (param, type)."""
import re


def signatures(path="capi/tools/proj_h_functions.rs.txt"):
    t = open(path, encoding="utf-8").read()
    out = {}
    for m in re.finditer(r"pub fn (\w+)\(", t):
        depth, end = 1, m.end()
        while depth:
            depth += {"(": 1, ")": -1}.get(t[end], 0)
            end += 1
        body = t[m.end() : end - 1]
        args = []
        level, cur = 0, ""
        for ch in body:
            if ch in "(<":
                level += 1
            elif ch in ")>":
                level -= 1
            if ch == "," and level == 0:
                args.append(cur)
                cur = ""
            else:
                cur += ch
        if cur.strip():
            args.append(cur)
        params = []
        for a in args:
            n, _, ty = a.partition(":")
            params.append((n.strip(), " ".join(ty.split())))
        tail = t[end : t.index(";", end)].strip()
        out[m.group(1)] = (params, tail[2:].strip() if tail.startswith("->") else None)
    return out
