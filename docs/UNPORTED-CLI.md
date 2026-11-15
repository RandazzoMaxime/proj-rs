# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, `proj`, and `invproj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_cs2cs_init_epsg.yaml#1` | cs2cs | expected -0.1283539405	51.5039908276 0.0000000000 got 0d7'36.292"W	51d30'12.528"N 0.000 | EPSG:27700 by +init, lon/lat degrees (ref cs2cs 9.9.0) |
| `test_cs2cs_init_epsg.yaml#2` | cs2cs | expected 530000.0008794245	179999.9995060905 0.0000000000 got 529887.09	180053.94 0.00 | EPSG:4326 by +init to EPSG:27700 by +init |
| `test_cs2cs_init_epsg.yaml#4` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: no transformation from EPS | Mixed pair, EPSG: source and +init= target |
| `test_cs2cs_init_epsg.yaml#5` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: no transformation from +in | Mixed pair, +init= source and EPSG: target |
| `test_cs2cs_init_epsg.yaml#10` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: no transformation from EPS | South oriented Transverse Mercator, mixed pair |
