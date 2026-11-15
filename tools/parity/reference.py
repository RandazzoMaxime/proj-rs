"""Binaires de la référence PROJ 9.9.0 et définitions de projections."""

import os
import re
import subprocess

from common import ROOT


class Reference:
    """Lance les binaires de la référence avec l'environnement de la suite."""

    def __init__(self, ref_dir, grids_dir=None):
        self.ref = ref_dir
        self.env = os.environ.copy()
        search = f"{ref_dir}/build/data:{grids_dir}" if grids_dir \
            else f"{ref_dir}/build/data:{ROOT}/data:{ROOT}/data/tests"
        self.env.update({
            "PROJ_DATA": search,
            "PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY": "YES",
            "PROJ_NETWORK": "OFF",
            "LD_LIBRARY_PATH": f"{ref_dir}/build/lib",
        })

    def run(self, tool, args, lines):
        text = "".join(f"{line}\n" for line in lines)
        proc = subprocess.run([f"{self.ref}/build/bin/{tool}", *args], input=text,
                              capture_output=True, text=True, env=self.env, check=False)
        return proc.returncode, proc.stdout.splitlines()

    def grid_free(self, src, dst):
        proc = subprocess.run([f"{self.ref}/build/bin/projinfo", "-s", src, "-t", dst],
                              capture_output=True, text=True, env=self.env, check=False)
        return proc.returncode == 0 and "grid missing" not in proc.stdout


def projection_defs(ids):
    """Définitions de `bench_definition` (src/ported.rs) ; sinon `+proj=ID +ellps=WGS84`."""
    ported = (ROOT / "src/ported.rs").read_text(encoding="utf-8")
    start = ported.index("pub fn bench_definition")
    body = ported[start:ported.index("\n}\n", start)]
    table = {}
    pattern = re.compile(r'((?:"\w+"\s*\|\s*)*"\w+")\s*=>\s*(?:\{\s*)?(?:format!\(\s*)?"([^"]*)"')
    for names_raw, template in pattern.findall(body):
        if any(s in template for s in ("grids=", "model=", "file=", "tests/", "urn:", "pipeline")):
            continue
        for name in re.findall(r'"(\w+)"', names_raw):
            table.setdefault(name, template.replace("{name}", name))
    return {pid: table.get(pid, f"+proj={pid} +ellps=WGS84") for pid in ids}
