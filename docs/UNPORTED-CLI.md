# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, and `proj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_cs2cs_various.yaml#29` | cs2cs | expected 0.00000	2.05479 0.00000 got 0.00000	3.39449 0.00000 | - |
| `test_cs2cs_various.yaml#30` | cs2cs | no numbers expected *	* inf got *	* inf | Test healpix inverse projection on ellipsoid |
| `test_cs2cs_various.yaml#31` | cs2cs | expected 0.00000	41.81031 0.00000 got 0.00000	23.80050 0.00000 | - |
| `test_cs2cs_various.yaml#37` | cs2cs | expected 0.00000	2.05479 0.00000 got 0.00000	3.39449 0.00000 | Test rHEALPix forward projection on ellipsoid north=0 south=0 |
| `test_cs2cs_various.yaml#38` | cs2cs | expected 0.00000	2.05479 0.00000 got 0.00000	3.39449 0.00000 | Test rHEALPix forward projection on ellipsoid north=1 south=1 |
| `test_cs2cs_various.yaml#39` | cs2cs | expected 0.00000	41.81031 0.00000 got 0.00000	23.80050 0.00000 | Test rHEALPix inverse projection on ellipsoid north=0 south=0 |
| `test_cs2cs_various.yaml#40` | cs2cs | expected 0.00000	41.81031 0.00000 got 0.00000	23.80050 0.00000 | Test rHEALPix inverse projection on ellipsoid north=1 south=1 |
| `test_cs2cs_various.yaml#57` | cs2cs | no numbers expected *	* inf got *	* inf | Check inverse error handling with ob_tran (#225) |
| `test_cs2cs_various.yaml#59` | cs2cs | expected 595710.3731286	5357598.4645652 0.0000000 got 595625.7338097	5357535.8096752 0.0000000 | Test MGI datum gives expected results (#207) |
| `test_cs2cs_various.yaml#71` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: unknown +proj=sch program abnormally terminated  | Test SCH forward projection |
| `test_cs2cs_various.yaml#72` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: unknown +proj=sch program abnormally terminated  | Test SCH inverse projection |
| `test_cs2cs_various.yaml#86` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `IAU_2015:4993 | Check we allow ellipsoids up to a 0.59% relative difference in size |
| `test_cs2cs_various.yaml#88` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `ITRF1993` pro | Test ITRF2000 to ITRF1993 |
| `test_cs2cs_various.yaml#89` | cs2cs | expected -132.0000000	46.0000000 0.0000000 got -2.3038346	0.8028515 0.0000000 | Check ob_tran with o_proj=longlat (#1525) |
| `test_cs2cs_various.yaml#90` | cs2cs | expected -112.0000000	46.0000000 0.0000000 got 39.9148994	64.3941424 0.0000000 | Test inverse handling |
| `test_cs2cs_various.yaml#91` | cs2cs | expected -20148827.8335825	6274861.3940066 0.0000000 got 19926188.8519960	6274861.3940066 0.0000000 | - |
| `test_cs2cs_various.yaml#92` | cs2cs | expected -20148827.8335825	6274861.3940066 0.0000000 got 19926188.8519960	6274861.3940066 0.0000000 | - |
| `test_cs2cs_various.yaml#95` | cs2cs | expected 49d0'0.002"N	2dE 0.018 got 49d11'37"E	0d59'43.836"N -6.570 | Test effect of --authority (GH-2442) - use the EPSG:8076 operation |
| `test_cs2cs_various.yaml#96` | cs2cs | expected 49dN	2dE 0.000 got 49d11'37"E	0d59'43.836"N -6.570 | Test effect of --authority (GH-2442) - a no-op |
| `test_cs2cs_various.yaml#97` | cs2cs | exit 0 expected 3  | Test effect of --accuracy |
| `test_cs2cs_various.yaml#98` | cs2cs | stderr text mismatch expected:  cannot initialize transformation cause: (null) program abnormally terminated  got:  cannot initialize transformation cause: The  | Test effect of --no-ballpark |
| `test_cs2cs_various.yaml#99` | cs2cs | expected 49d59'59.36"N	179d59'52.133"W 0.000 got 49d59'57.924"N	179d59'52.075"W -6.772 | Check that we can use a transformation spanning the antimeridian (should use Pulkovo 1942 to WGS 84 (20)) |
| `test_cs2cs_various.yaml#100` | cs2cs | expected 49d59'59.36"N	179d59'52.133"W 0.000 got 49d59'57.924"N	179d59'52.076"W -6.772 | - |
| `test_cs2cs_various.yaml#102` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+577 | Check that we promote CRS specified by name to 3D when the other one is 3D |
| `test_cs2cs_various.yaml#103` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+577 | - |
| `test_cs2cs_various.yaml#110` | cs2cs | stderr text mismatch expected: Attempt to use coordinate operation NTF to ETRS89-FRA [RGF93 v1] (1) failed. Grid fr_ign_gr3df97a.tif is not available. Consult h | Test cs2cs --only-best (grid missing) for NTF to RGF93 |
| `test_cs2cs_various.yaml#111` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+577 | Test cs2cs --only-best (working) for EPSG:4326+5773 to EPSG:4979 |
| `test_cs2cs_various.yaml#112` | cs2cs | expected 30dS	152dE 0.000 got 29d59'56.497"S	152d0'1.523"E -0.217 | Test cs2cs --only-best (working) for EPSG:4326 to GDA94 |
| `test_cs2cs_various.yaml#113` | cs2cs | expected 30dS	152dE 0.000 got 30d0'3.503"S	151d59'58.477"E 0.219 | Test cs2cs --only-best (working) for GDA94 to EPSG:4326 |
| `test_cs2cs_various.yaml#114` | cs2cs | expected 30dS	152dE 0.000 got 4881228d19'8.549"W	2595348d16'7.796"N -3170278.941 | Test cs2cs --only-best (working) for EPSG:4326 to GDA2020 |
| `test_cs2cs_various.yaml#115` | cs2cs | expected 30dS	152dE 0.000 got 90dS	101d9'51.13"E -6356752.312 | Test cs2cs --only-best (working) for GDA2020 to EPSG:4326 |
| `test_cs2cs_various.yaml#116` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+385 | Test cs2cs (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#117` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+385 | Test cs2cs --only-best=no (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#118` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+385 | Test cs2cs --only-best (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#119` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+385 | Test cs2cs --only-best=yes (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#120` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+385 | Test cs2cs (grid missing) with PROJ_ONLY_BEST_DEFAULT=YES for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#121` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+385 | Test cs2cs (grid missing) with only_best_default=on in proj.ini |
| `test_cs2cs_various.yaml#122` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: EPSG datum 6267 to 6269 ne | Test cs2cs --only-best (grid missing) |
| `test_cs2cs_various.yaml#123` | cs2cs | stderr text mismatch expected: Attempt to use coordinate operation Inverse of WGS 84 to EGM2008 height (1) failed. Grid us_nga_egm08_25.tif is not available. Co | Test cs2cs --only-best --no-ballpark (grid missing) |
| `test_cs2cs_various.yaml#124` | cs2cs | expected 1402288.54	5076296.64 0.00 got 1402285.98	5076292.42 0.00 | Test cs2cs (grid missing) with scenario of GH-3607 that we are using the 'NAD27 to WGS 84 (6)' DMA-ConusW transformation |
| `test_cs2cs_various.yaml#125` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: EPSG datum 6269 to 6267 ne | Test cs2cs (grid missing) with scenario of GH-3613 that we are using a WGS 84 intermediate to do NAD27 to NAD83 |
| `test_cs2cs_various.yaml#126` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+577 | Test cs2cs (grid missing) with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#127` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+577 | Test cs2cs (grid missing) --only-best=no with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#128` | cs2cs | stderr text mismatch expected: Attempt to use coordinate operation Inverse of WGS 84 to EGM96 height (1) + ETRS89-ESP [REGENTE] to Alicante height (1) using ETR | Test cs2cs (grid missing) --only-best with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#130` | cs2cs | expected 4003461.55	705832.08 4898267.79 got 3995986.86	4846735.07 0.00 | Test cs2cs geographic -> geocentric using BETA2007.gsb grid |
| `test_cs2cs_various.yaml#131` | cs2cs | expected 50d30'N	10dE -0.001 got 10d0'0.078"E	0d0'21.509"S 4898267.790 | Test cs2cs geocentric -> geographic using BETA2007.gsb grid |
| `test_cs2cs_various.yaml#132` | cs2cs | expected 4003461.37	705832.04 4898267.94 got 3995986.86	4846735.07 0.00 | Test cs2cs geographic -> geocentric, normally using BETA2007.gsb grid but missing |
| `test_cs2cs_various.yaml#133` | cs2cs | expected 50d30'N	10dE -0.002 got 10d0'0.078"E	0d0'21.509"S 4898267.940 | Test cs2cs geocentric -> geographic, normally using BETA2007.gsb grid but missing |
| `test_cs2cs_various.yaml#134` | cs2cs | expected 4189881.02	146313.87 4790558.75 got 4182107.12	4810218.62 0.00 | Test cs2cs geographic -> geocentric outside BETA2007.gsb grid |
| `test_cs2cs_various.yaml#135` | cs2cs | expected 49dN	2dE 0.002 got 2d0'0.192"E	0d0'20.859"S 4790558.750 | Test cs2cs geocentric -> geographic outside BETA2007.gsb grid |
| `test_cs2cs_various.yaml#136` | cs2cs | expected 299905.060	4499796.515 0.000 got 299910.460	4499798.714 0.000 | Test Similarity Transformation (example from EPSG Guidance Note 7.2) |
| `test_cs2cs_various.yaml#137` | cs2cs | expected 300000.000	4500000.000 0.000 got 299994.600	4499997.801 0.000 | Test inverse of Similarity Transformation (example from EPSG Guidance Note 7.2) |
| `test_cs2cs_various.yaml#138` | cs2cs | expected 476763.303	95620.222 0.000 got 476763.119	95620.110 46.846 | Test Similarity Transformation through CompoundCRS |
| `test_cs2cs_various.yaml#139` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:3912+577 | - |
| `test_cs2cs_various.yaml#140` | cs2cs | line count 1 vs 2 expected -30.00000099	149.99999956 0.00031880  got -30.00177330	149.99964534 0.00255674 Input coordinates lack a coordinate epoch, whereas the | Test --s_epoch |
| `test_cs2cs_various.yaml#141` | cs2cs | line count 1 vs 2 expected -30 150	-29.99999901	150.00000044 -0.00031879  got -30 150	-29.99822670	150.00035466 -0.00255674 Input coordinates lack a coordinate  | Test --t_epoch |
| `test_cs2cs_various.yaml#142` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: `+proj=topocentric +datum= | - |
| `test_cs2cs_various.yaml#143` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4559+575 | Test cs2cs EPSG:5488 (RGAF09) to EPSG:4559+5757 (RRAF 1991 / UTM zone 20N + Guadeloupe 1988 height) |
| `test_projinfo.yaml#6` | projinfo | exit 1 expected 0 projinfo: cannot write dialect SQL usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and | - |
| `test_projinfo.yaml#7` | projinfo | exit 1 expected 0 projinfo: cannot write dialect SQL usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and | - |
| `test_projinfo.yaml#10` | projinfo | stderr text mismatch expected: Too many parameters: EPSG:32631  got: projinfo: Too many parameters: EPSG:4327  | - |
| `test_projinfo.yaml#11` | projinfo | text mismatch expected: Candidate operations found: 1 Note: using '--spatial-test intersects' would bring more results (10) ------------------------------------ | - |
| `test_projinfo.yaml#12` | projinfo | text mismatch expected: Candidate operations found: 1 Note: using '--spatial-test intersects' would bring more results (10) ------------------------------------ | - |
| `test_projinfo.yaml#13` | projinfo | text mismatch expected: Candidate operations found: 9 DERIVED_FROM(EPSG):1313, NAD27 to NAD83 (4), 1.5 m, Canada - onshore - Alberta; British Columbia; Manitoba | - |
| `test_projinfo.yaml#14` | projinfo | text mismatch expected: Candidate operations found: 10 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):1313, NAD27 to NAD83 (4), 1.5  | - |
| `test_projinfo.yaml#15` | projinfo | text mismatch expected: Candidate operations found: 1 Note: using '--spatial-test intersects' would bring more results (2) EPSG:1626, ED50 to ETRS89 (4), 1.0 m, | - |
| `test_projinfo.yaml#16` | projinfo | text mismatch expected: Candidate operations found: 1 Note: using '--spatial-test intersects' would bring more results (9) unknown id, Inverse of UTM zone 31N + | - |
| `test_projinfo.yaml#17` | projinfo | text mismatch expected: Candidate operations found: 1 EPSG:1626, ED50 to ETRS89 (4), 1.0 m, Denmark - onshore.  got: Candidate operations found: 14 EPSG:1588, E | - |
| `test_projinfo.yaml#18` | projinfo | text mismatch expected: Candidate operations found: 1 EPSG:1626, ED50 to ETRS89 (4), 1.0 m, Denmark - onshore.  got: Candidate operations found: 14 EPSG:1588, E | - |
| `test_projinfo.yaml#19` | projinfo | exit 0 expected 1  | several match |
| `test_projinfo.yaml#20` | projinfo | exit 0 expected 1  | - |
| `test_projinfo.yaml#21` | projinfo | exit 0 expected 1  | - |
| `test_projinfo.yaml#22` | projinfo | text mismatch expected: Warning: object is deprecated Alternative non-deprecated CRS:   EPSG:3003  PROJ.4 string: +proj=tmerc +lat_0=0 +lon_0=9 +k=0.9996 +x_0=1 | Testing deprecated CRS |
| `test_projinfo.yaml#23` | projinfo | exit 1 expected 0 projinfo: GEOGCS should have a PRIMEM node  | Testing non compliant WKT1 |
| `test_projinfo.yaml#24` | projinfo | text mismatch expected: PROJ.4 string: +proj=utm +zone=32 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs  got: PROJ.4 string: +proj=utm +zone=3 | Testing CRS with towgs84 |
| `test_projinfo.yaml#25` | projinfo | text mismatch expected: Candidate operations found: 2 ------------------------------------- Operation No. 1:  INVERSE(DERIVED_FROM(PROJ)):EPSG_4977_TO_EPSG_5613 | Testing RH2000 height to ETRS89-SWE [SWEREF 99] |
| `test_projinfo.yaml#26` | projinfo | text mismatch expected: Candidate operations found: 3 ------------------------------------- Operation No. 1:  unknown id, Inverse of NAD83(2011) to NAVD88 heigh | Testing NAD83(2011) + NAVD88 height -> NAD83(2011) |
| `test_projinfo.yaml#27` | projinfo | text mismatch expected: Candidate operations found: 2 ------------------------------------- Operation No. 1:  INVERSE(DERIVED_FROM(EPSG)):10000, Inverse of ETRS | Testing NGF IGN69 height to RGF93 |
| `test_projinfo.yaml#28` | projinfo | text mismatch expected: PROJ.4 string: +proj=utm +zone=31 +datum=WGS84 +units=m +no_defs +type=crs  WKT2:2019 string: PROJCRS["WGS 84 / UTM zone 31N",     BASEG | - |
| `test_projinfo.yaml#29` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, WGS 84 to EGM96 height (1), 1 m, World.  got: Candidate operations found: 0   | - |
| `test_projinfo.yaml#30` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Inverse of WGS 84 to EGM96 height (1), 1 m, World.  got: Candidate operations found: 0   | - |
| `test_projinfo.yaml#31` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Inverse of UTM zone 31N + Inverse of Null geographic offset from WGS 84 to WGS 84, 0 m, World. | - |
| `test_projinfo.yaml#32` | projinfo | text mismatch expected: Candidate operations found: 3 unknown id, Inverse of UTM zone 31N + WGS 84 to EGM2008 height (1), 0.113 m, World. unknown id, Inverse of | - |
| `test_projinfo.yaml#33` | projinfo | text mismatch expected: +proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +e | Undocumented option: --normalize-axis-order |
| `test_projinfo.yaml#34` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Ballpark geocentric translation from ETRS89 to WGS 84, unknown accuracy, World, has ballpark t | WGS 84 to ETRS89 (2) uses a transformation method not supported by PROJ currently (time-specific Helmert), and thus must be sorted last |
| `test_projinfo.yaml#35` | projinfo | exit 1 expected 0 projinfo: missing [  | - |
| `test_projinfo.yaml#36` | projinfo | text mismatch expected: Candidate operations found: 19 unknown id, AGD66 to WGS 84 (18) + WGS 84 to WGS 84 (G1762), 5 m, Australia - offshore including EEZ. unk | - |
| `test_projinfo.yaml#37` | projinfo | text mismatch expected: Candidate operations found: 4 unknown id, Inverse of 3-degree Gauss-Kruger zone 3 + DHDN to ETRS89 (9), 0.1 m, Germany - Baden-Wurttembe | - |
| `test_projinfo.yaml#38` | projinfo | text mismatch expected: Candidate operations found: 1 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):5656, GDA94 to AHD height (49), | - |
| `test_projinfo.yaml#39` | projinfo | text mismatch expected: Candidate operations found: 1 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):8451, GDA2020 to AHD height (1) | - |
| `test_projinfo.yaml#40` | projinfo | exit 1 expected 0 projinfo: unrecognized option -k usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and W | - |
| `test_projinfo.yaml#41` | projinfo | exit 1 expected 0 projinfo: unrecognized option -k usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and W | - |
| `test_projinfo.yaml#42` | projinfo | exit 1 expected 0 projinfo: unrecognized option -k usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and W | - |
| `test_projinfo.yaml#43` | projinfo | exit 1 expected 0 projinfo: unrecognized option -k usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and W | - |
| `test_projinfo.yaml#44` | projinfo | exit 1 expected 0 projinfo: unrecognized option -k usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and W | - |
| `test_projinfo.yaml#45` | projinfo | exit 1 expected 0 projinfo: unrecognized option -k usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and W | - |
| `test_projinfo.yaml#46` | projinfo | exit 1 expected 0 projinfo: missing [  | - |
| `test_projinfo.yaml#47` | projinfo | exit 1 expected 0 projinfo: unrecognized option -k usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and W | - |
| `test_projinfo.yaml#48` | projinfo | exit 1 expected 0 projinfo: missing EPSG code usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and WKT2:2 | Hard to test content |
| `test_projinfo.yaml#49` | projinfo | exit 1 expected 0 projinfo: missing EPSG code usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and WKT2:2 | - |
| `test_projinfo.yaml#50` | projinfo | exit 1 expected 0 projinfo: missing EPSG code usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and WKT2:2 | Hard to test content |
| `test_projinfo.yaml#51` | projinfo | text mismatch expected: Candidate operations found: 0  got: Candidate operations found: 0 -------------------------------------   | - |
| `test_projinfo.yaml#52` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#53` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#54` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#55` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#56` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#57` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#58` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=tinshift +file=fi_nls_ykj_etrs35fin.json  got: Candidate operations found | - |
| `test_projinfo.yaml#59` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#60` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#61` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#62` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#63` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=tinshift +file=fi_nls_n43_n60.json   +step +proj=tinshift +file=fi_nls_n6 | - |
| `test_projinfo.yaml#64` | projinfo | text mismatch expected: +proj=pipeline   +step +inv +proj=tinshift +file=fi_nls_ykj_etrs35fin.json   +step +inv +proj=tinshift +file=fi_nls_n60_n2000.json   +st | - |
| `test_projinfo.yaml#65` | projinfo | text mismatch expected: Candidate operations found: 2 EPSG:10894, ITRF2014 to ETRS89-DNK (1), 0.006 m, Denmark - onshore and offshore., time-dependent operation | Quick check of NKG transformations |
| `test_projinfo.yaml#66` | projinfo | exit 1 expected 0 projinfo: missing EPSG code usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and WKT2:2 | - |
| `test_projinfo.yaml#67` | projinfo | exit 1 expected 0 projinfo: CRS not found (HOBU:XXXX)  | - |
| `test_projinfo.yaml#68` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Inverse of UTM zone 30N + ED50 to ETRS89 (12) + UTM zone 30N, 0.2 m, Spain - mainland, Baleari | Checks that ED50 to ETRS89 (12) is in the output (superseded transformation, but replacements has unknown grid) |
| `test_projinfo.yaml#72` | projinfo | text mismatch expected: EPSG:5782 "Alicante height" got: EPSG:10918 "COH88 2025 (NAVD88) height" EPSG:10919 "COH88 2025 (NAVD88) height (ftUS)" EPSG:5703 "NAVD8 | - |
| `test_projinfo.yaml#73` | projinfo | text mismatch expected: EPSG:5782 "Alicante height" got: EPSG:10918 "COH88 2025 (NAVD88) height" EPSG:10919 "COH88 2025 (NAVD88) height (ftUS)" EPSG:5703 "NAVD8 | - |
| `test_projinfo.yaml#74` | projinfo | text mismatch expected: EPSG:4258 "ETRS89" EPSG:9398 "Tenerife height" EPSG:9505 "ETRS89 + Alicante height"  got: EPSG:4258 "ETRS89" EPSG:5703 "NAVD88 height" E | - |
| `test_projinfo.yaml#75` | projinfo | text mismatch expected: EPSG:9398 "Tenerife height" EPSG:9505 "ETRS89 + Alicante height"  got: EPSG:4258 "ETRS89" EPSG:5703 "NAVD88 height" EPSG:9398 "Tenerife  | - |
| `test_projinfo.yaml#76` | projinfo | text mismatch expected: EPSG:9398 "Tenerife height" EPSG:9505 "ETRS89 + Alicante height"  got: EPSG:4258 "ETRS89" EPSG:5703 "NAVD88 height" EPSG:9398 "Tenerife  | - |
| `test_projinfo.yaml#79` | projinfo | text mismatch expected: EPSG:4978 "WGS 84" EPSG:4979 "WGS 84"  got: EPSG:4978 "WGS 84"  | - |
| `test_projinfo.yaml#80` | projinfo | text mismatch expected: EPSG:8449 "NAD83(FBN)" [deprecated]  got:  | - |
| `test_projinfo.yaml#81` | projinfo | text mismatch expected: EPSG:6512 "NAD83(2011) / Missouri East" got: EPSG:2129 "NZGD2000 / Mount York 2000" EPSG:2260 "NAD83 / New York East (ftUS)" EPSG:2261 " | - |
| `test_projinfo.yaml#82` | projinfo | text mismatch expected: EPSG:2154 "ETRS89-FRA [RGF93 v1] / Lambert-93" got: EPSG:2154 "ETRS89-FRA [RGF93 v1] / Lambert-93" EPSG:22211 "NAD83(CSRS)v2 / UTM zone  | Test case where --area has several matches |
| `test_projinfo.yaml#83` | projinfo | text mismatch expected: +proj=tmerc +lat_0=0 +lon_0=21 +k=0.9999 +x_0=500000 +y_0=-4000000 +ellps=bessel +towgs84=521.748,229.489,590.921,4.029,4.488,-15.521,-9 | Test North-Macedonia CRS for which there are several MGI 1901 -> WGS 84 Helmert transformations but only one matches the extent of the CRS |
| `test_projinfo.yaml#84` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Null geographic offset from NAD83(CSRS)v7 (geog2D) to NAD83(CSRS)v7 (geog3D) + Canada velocity | - |
| `test_projinfo.yaml#85` | projinfo | text mismatch expected: +proj=noop got: Candidate operations found: 1 ------------------------------------- Operation No. 1:  EPSG:10620, CGVD28 height to CGVD2 | Test that "CGVD28 height" to "CGVD28(HTv2.0) height" is a no-op |
| `test_projinfo.yaml#86` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=vgridshift +grids=us_nga | Test custom records for WGS 84 realizations to EGM2008 (https://github.com/OSGeo/PROJ/issues/4362) |
| `test_projinfo.yaml#87` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=cart +ellps=WGS84   +ste | Test 'WGS 84 (Gxxx) + EGM2008 height' to 'WGS 84 (Gyyy) + EGM2008 height' |
| `test_projinfo.yaml#88` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRF2005 (geog3D) to ETRF2005 (geocentric) + Inverse of ITRF2005 to ETRF2005 ( | Test that ETRF2000 to ETRF2014 only goes through ITRF2000, ITRF2008 or ITRF2014, and not ITRF9x or ITRF>2020 |
| `test_projinfo.yaml#89` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRS89-POL [PL-ETRF2000] (geog2D) to ETRS89-POL [PL-ETRF2000] (geocentric) + I | Test that ETRF2000-PL to ETRF2014 is not negatively affected by https://github.com/OSGeo/PROJ/pull/4364 changes (the current actual result is not golden in any way, using ETRF2000 to ETRF2014 could probably make sense) |
| `test_projinfo.yaml#90` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Conversion from WGS 84 (G1150) (geog2D) to WGS 84 (G1150) (geocentric) + WGS 84 (G1150) to WGS | Test WGS 84 (G1150) to WGS 84 (G2296) |
| `test_projinfo.yaml#91` | projinfo | text mismatch expected: -o -k --summary -q --area --bbox --spatial-test --crs-extent-use --grid-check --pivot-crs --show-superseded --hide-ballpark --accuracy - | - |
| `test_projinfo.yaml#95` | projinfo | text mismatch expected: always if_no_direct_transformation never EPSG: ESRI: IAU_2015: IGNF: NKG: NRCAN: OGC: PROJ:  got:  always if_no_direct_transformation ne | - |
| `test_projinfo.yaml#96` | projinfo | text mismatch expected: EPSG: ESRI: IAU_2015: IGNF: NKG: NRCAN: OGC: PROJ:  got:  EPSG: ESRI: IAU_2015: IGNF: NKG: NRCAN: OGC: PROJ:  | - |
| `test_projinfo.yaml#97` | projinfo | text mismatch expected: NKG: NKG_ETRF14 NKG_ETRF14 NKG_ETRF14 got:  | - |
| `test_projinfo.yaml#98` | projinfo | text mismatch expected: CRS27\ --\ NAD27\ (CRS27) CRS83\ --\ NAD83\ (CRS83) CRS84\ --\ WGS\ 84\ (CRS84) CRS84h\ --\ WGS\ 84\ longitude-latitude-height  got:  | - |
| `test_projinfo.yaml#99` | projinfo | text mismatch expected: 4322\ --\ WGS\ 72 4324\ --\ WGS\ 72BE 4326\ --\ WGS\ 84  got:  4322\ --\ WGS\ 72 4324\ --\ WGS\ 72BE 4326\ --\ WGS\ 84  | - |
| `test_projinfo.yaml#100` | projinfo | text mismatch expected: 4326  got:  4326  | - |
| `test_projinfo.yaml#101` | projinfo | text mismatch expected: EGM2008\ height EGM96\ height EGM84\ height  got:  | - |
| `test_projinfo.yaml#102` | projinfo | text mismatch expected: Lambert-93 CC42 CC43 CC44 CC45 CC46 CC47 CC48 CC49 CC50 Lambert-93\ +\ NGF-IGN69\ height Lambert-93\ +\ NGF-IGN78\ height  got:  | - |
| `test_projinfo.yaml#103` | projinfo | text mismatch expected: NGVD29\ height\ (ftUS) NAVD88\ depth\ (ftUS) NGVD29\ depth\ (ftUS) NAVD88\ height\ (ftUS) NGVD29\ height\ (m) MSL\ height\ (ftUS) MSL\ d | - |
| `test_projinfo.yaml#104` | projinfo | text mismatch expected: EGM2008\ height MSL\ height MSL\ depth EGM96\ height EGM84\ height Instantaneous\ Water\ Level\ height Instantaneous\ Water\ Level\ dept | - |
| `test_projinfo.yaml#105` | projinfo | text mismatch expected: +proj=affine +xoff=550015 +yoff=8780001  got: Candidate operations found: 1 ------------------------------------- Operation No. 1:  EPSG | - |
| `test_projinfo.yaml#106` | projinfo | text mismatch expected: +proj=noop  got: Candidate operations found: 0 -------------------------------------   | - |
| `test_projinfo.yaml#107` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Ballpark geographic offset from NAD83(CSRS) to NAD83(CSRS)v2, unknown accuracy, Canada - onsho | Test bugfix of https://github.com/OSGeo/PROJ/issues/4464 (NAD83(CSRS) to NAD83(CSRS)v2 should not go through NAD83) |
| `test_projinfo.yaml#108` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Inverse of Saba to WGS 84 (1) + Saba to Saba height (1) + Saba Transverse Mercator 2020, 1.2 m | Test that EPSG:4979 to EPSG:10645 does not include a non-sensical 'Inverse of Inverse of BES2020 Saba to Saba height (1) + Inverse of Saba to WGS 84 (1) + Saba Transverse Mercator 2020' |
| `test_projinfo.yaml#109` | projinfo | exit 1 expected 0 projinfo: missing [  | Test that 'projinfo EGM2008' by default resolves to the datum |
| `test_projinfo.yaml#110` | projinfo | exit 1 expected 0 projinfo: unrecognized option -k usage: projinfo [-o formats] [-q] [--version] EPSG:code -o: PROJ, WKT2:2019, PROJJSON, or default (PROJ and W | Test that 'projinfo -k crs EGM2008' resolves to the CRS |
| `test_proj.yaml#3` | proj | exit 1 expected 0 proj: crs is not built  | Test projection factors on projected CRS with non-Greenwhich prime meridian |
| `test_proj.yaml#4` | proj | text mismatch expected: 500000.00	0.00	<0.9996 0.9996 0.9992 0 0.9996 0.9996> got: 499999.41	-0.46	<0.9996 0.9996 0.9992 0 0.9996 0.9996>  | Test projection factors on compound CRS with a projected CRS |
