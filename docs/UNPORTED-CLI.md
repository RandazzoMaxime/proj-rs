# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, `proj`, and `invproj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_cs2cs_datumfile.yaml#1` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: illegal value for +grids program abnormally terminated  | 1st through ntv1, 2nd through conus |
| `test_cs2cs_datumfile.yaml#2` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: unknown parameter +with program abnormally terminated  | As above, but without ntv1 everything goes through conus file |
| `test_cs2cs_datumfile.yaml#3` | cs2cs | no numbers expected 40d30'0.368"N	59d59'56.617"W 0.000 got *	* inf | Test --bbox -141.01,40.04,-47.74,83.17 NAD27 NAD83 (using NTv2) |
| `test_cs2cs_datumfile.yaml#10` | cs2cs | expected 2.00	49.00 0.00 got 2dE	49dN -45.064 | WGS84 (2D) -> WGS84+EGM96 |
| `test_cs2cs_datumfile.yaml#11` | cs2cs | expected 2.00	49.00 -45.06 got 2dE	49dN -45.064 | WGS84 (3D) -> WGS84+EGM96 |
| `test_cs2cs_datumfile.yaml#12` | cs2cs | expected 2.00	49.00 -45.06 got 2dE	49dN -45.064 | WGS84 (2D), promoted to 3D -> WGS84+EGM96 |
| `test_cs2cs_ignf.yaml#1` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#2` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#3` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#4` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#5` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#6` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#7` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#8` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#9` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#10` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#11` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#12` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#13` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#14` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#15` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#16` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#17` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#18` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#19` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#20` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#21` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#22` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#23` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#24` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#25` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#26` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#27` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#28` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#29` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#30` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#31` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#32` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#33` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ignf.yaml#34` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: init file is not built program abnormally terminated  | - |
| `test_cs2cs_ntv2.yaml#2` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: illegal value for +grids program abnormally terminated  | Try with NTv2 and NTv1 together ... falls back to NTv1 |
