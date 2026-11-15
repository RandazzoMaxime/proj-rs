# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, `proj`, and `invproj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_cs2cs_init_epsg.yaml#10` | cs2cs | expected 25.0226371876	26.7538956993 0.0000000000 got 4d58'38.506"E	26d45'14.025"S 0.000 | South oriented Transverse Mercator, mixed pair |
