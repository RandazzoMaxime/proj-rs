"""Catalogue des CRS EPSG non dépréciés, lu dans la proj.db de la référence."""

import sqlite3
from collections import defaultdict

from common import GEOG_KINDS, METRE, lon_width


class Catalog:
    """CRS EPSG non dépréciés, en degrés ou en mètres, avec leur emprise (usage)."""

    def __init__(self, db_path):
        db = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
        self.axes = defaultdict(list)
        for cs_auth, cs_code, abbrev, uom in db.execute(
            "select coordinate_system_auth_name, coordinate_system_code, abbrev, uom_code "
            "from axis where uom_auth_name='EPSG' "
            "order by coordinate_system_auth_name, coordinate_system_code, coordinate_system_order"
        ):
            self.axes[(cs_auth, str(cs_code))].append((abbrev, int(uom)))
        extents = {}
        for table, o_auth, o_code, south, north, west, east in db.execute(
            "select u.object_table_name, u.object_auth_name, u.object_code, x.south_lat, "
            "x.north_lat, x.west_lon, x.east_lon from usage u join extent x "
            "on x.auth_name=u.extent_auth_name and x.code=u.extent_code "
            "where x.deprecated=0 and x.south_lat is not null order by u.rowid"
        ):
            area = (north - south) * lon_width(west, east)
            key = (table, o_auth, str(o_code))
            if key not in extents or area < extents[key][0]:
                extents[key] = (area, (south, north, west, east))
        self.extents = {key: value[1] for key, value in extents.items()}
        self.by_kind = defaultdict(list)
        self.by_ref = {}
        self._load_geodetic(db)
        self._load_projected(db)
        self._load_vertical(db)
        self._load_compound(db)

    def _uoms(self, cs_auth, cs_code):
        return [uom for _, uom in self.axes.get((cs_auth, str(cs_code)), [])]

    def _add(self, crs):
        self.by_kind[crs["kind"]].append(crs)
        self.by_ref[crs["ref"]] = crs

    def _load_geodetic(self, db):
        for auth, code, typ, cs_a, cs_c, d_a, d_c in db.execute(
            "select auth_name, code, type, coordinate_system_auth_name, coordinate_system_code, "
            "datum_auth_name, datum_code from geodetic_crs "
            "where deprecated=0 and auth_name='EPSG' order by code"
        ):
            if typ not in GEOG_KINDS:
                continue
            kind, uoms = GEOG_KINDS[typ]
            ax = self.axes.get((cs_a, str(cs_c)), [])
            if [uom for _, uom in ax] != uoms:
                continue
            self._add({
                "kind": kind, "ref": f"EPSG:{code}", "ext": self.extents.get(("geodetic_crs", auth, str(code))),
                "datum": (d_a, str(d_c)), "dims": len(uoms), "lat_first": ax[0][0] in ("Lat", "N"),
            })

    def _load_projected(self, db):
        for auth, code, cs_a, cs_c, b_c in db.execute(
            "select auth_name, code, coordinate_system_auth_name, coordinate_system_code, "
            "geodetic_crs_code from projected_crs where deprecated=0 and auth_name='EPSG' order by code"
        ):
            base = self.by_ref.get(f"EPSG:{b_c}")
            ext = self.extents.get(("projected_crs", auth, str(code)))
            if base is None or ext is None or self._uoms(cs_a, cs_c) != [METRE, METRE]:
                continue
            self._add({"kind": "proj", "ref": f"EPSG:{code}", "ext": ext, "datum": base["datum"],
                       "dims": 2, "lat_first": False})

    def _load_vertical(self, db):
        for auth, code, cs_a, cs_c in db.execute(
            "select auth_name, code, coordinate_system_auth_name, coordinate_system_code "
            "from vertical_crs where deprecated=0 and auth_name='EPSG' order by code"
        ):
            if self._uoms(cs_a, cs_c) == [METRE]:
                self._add({"kind": "vert", "ref": f"EPSG:{code}",
                           "ext": self.extents.get(("vertical_crs", auth, str(code))),
                           "datum": None, "dims": 1, "lat_first": False})

    def _load_compound(self, db):
        for auth, code, h_c, v_c in db.execute(
            "select auth_name, code, horiz_crs_code, vertical_crs_code from compound_crs "
            "where deprecated=0 and auth_name='EPSG' order by code"
        ):
            horiz = self.by_ref.get(f"EPSG:{h_c}")
            vert = self.by_ref.get(f"EPSG:{v_c}")
            ext = self.extents.get(("compound_crs", auth, str(code)))
            if horiz is None or vert is None or vert["kind"] != "vert" or ext is None:
                continue
            if horiz["kind"] not in ("geog2d", "proj"):
                continue
            self._add({"kind": "compound", "ref": f"EPSG:{code}", "ext": ext, "datum": horiz["datum"],
                       "dims": 3, "lat_first": horiz["lat_first"], "horiz": horiz["kind"],
                       "vert": f"EPSG:{v_c}"})
