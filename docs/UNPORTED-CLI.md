# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, and `proj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_cs2cs_various.yaml#59` | cs2cs | expected 595710.3731286	5357598.4645652 0.0000000 got 595625.7338097	5357535.8096752 0.0000000 | Test MGI datum gives expected results (#207) |
| `test_cs2cs_various.yaml#89` | cs2cs | expected -132.0000000	46.0000000 0.0000000 got -2.3038346	0.8028515 0.0000000 | Check ob_tran with o_proj=longlat (#1525) |
| `test_cs2cs_various.yaml#90` | cs2cs | expected -112.0000000	46.0000000 0.0000000 got 39.9148994	64.3941424 0.0000000 | Test inverse handling |
| `test_cs2cs_various.yaml#97` | cs2cs | exit 0 expected 3  | Test effect of --accuracy |
| `test_cs2cs_various.yaml#98` | cs2cs | exit 0 expected 3  | Test effect of --no-ballpark |
| `test_cs2cs_various.yaml#100` | cs2cs | expected 49d59'59.36"N	179d59'52.133"W 0.000 got 49d59'57.924"N	179d59'52.076"W -6.772 | - |
| `test_cs2cs_various.yaml#118` | cs2cs | no numbers expected *	* inf got 49dN	2dE 0.000 | Test cs2cs --only-best (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#119` | cs2cs | no numbers expected *	* inf got 49dN	2dE 0.000 | Test cs2cs --only-best=yes (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#120` | cs2cs | no numbers expected *	* inf got 49dN	2dE 0.000 | Test cs2cs (grid missing) with PROJ_ONLY_BEST_DEFAULT=YES for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#121` | cs2cs | no numbers expected *	* inf got 49dN	2dE 0.000 | Test cs2cs (grid missing) with only_best_default=on in proj.ini |
| `test_cs2cs_various.yaml#122` | cs2cs | no numbers expected *	* inf got 40dN	100dW 0.000 | Test cs2cs --only-best (grid missing) |
| `test_cs2cs_various.yaml#123` | cs2cs | exit 0 expected 3  | Test cs2cs --only-best --no-ballpark (grid missing) |
| `test_cs2cs_various.yaml#124` | cs2cs | expected 1402288.54	5076296.64 0.00 got 1402285.98	5076292.42 0.00 | Test cs2cs (grid missing) with scenario of GH-3607 that we are using the 'NAD27 to WGS 84 (6)' DMA-ConusW transformation |
| `test_cs2cs_various.yaml#125` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: EPSG datum 6269 to 6267 ne | Test cs2cs (grid missing) with scenario of GH-3613 that we are using a WGS 84 intermediate to do NAD27 to NAD83 |
| `test_cs2cs_various.yaml#126` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+577 | Test cs2cs (grid missing) with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#127` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4326+577 | Test cs2cs (grid missing) --only-best=no with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#128` | cs2cs | stderr text mismatch expected: Attempt to use coordinate operation Inverse of WGS 84 to EGM96 height (1) + ETRS89-ESP [REGENTE] to Alicante height (1) using ETR | Test cs2cs (grid missing) --only-best with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#130` | cs2cs | expected 4003461.55	705832.08 4898267.79 got 3995986.86	4846735.07 1100568.03 | Test cs2cs geographic -> geocentric using BETA2007.gsb grid |
| `test_cs2cs_various.yaml#132` | cs2cs | expected 4003461.37	705832.04 4898267.94 got 3995986.86	4846735.07 1100568.03 | Test cs2cs geographic -> geocentric, normally using BETA2007.gsb grid but missing |
| `test_cs2cs_various.yaml#134` | cs2cs | expected 4189881.02	146313.87 4790558.75 got 4182107.12	4810218.62 221502.98 | Test cs2cs geographic -> geocentric outside BETA2007.gsb grid |
| `test_cs2cs_various.yaml#135` | cs2cs | expected 49dN	2dE 0.002 got 2d0'0.12"E	49d0'3.647"N -50.127 | Test cs2cs geocentric -> geographic outside BETA2007.gsb grid |
| `test_cs2cs_various.yaml#139` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:3912+577 | - |
| `test_cs2cs_various.yaml#143` | cs2cs | exit 3 expected 0 Rel. 9.9.0, September 15th, 2026 <cs2cs>:  cannot initialize transformation cause: The underlying PROJ call failed: unknown CRS `EPSG:4559+575 | Test cs2cs EPSG:5488 (RGAF09) to EPSG:4559+5757 (RRAF 1991 / UTM zone 20N + Guadeloupe 1988 height) |
| `test_projinfo.yaml#10` | projinfo | stderr text mismatch expected: Too many parameters: EPSG:32631  got: Too many parameters: EPSG:4327  | - |
| `test_projinfo.yaml#13` | projinfo | text mismatch expected: Candidate operations found: 9 DERIVED_FROM(EPSG):1313, NAD27 to NAD83 (4), 1.5 m, Canada - onshore - Alberta; British Columbia; Manitoba | - |
| `test_projinfo.yaml#14` | projinfo | text mismatch expected: Candidate operations found: 10 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):1313, NAD27 to NAD83 (4), 1.5  | - |
| `test_projinfo.yaml#22` | projinfo | text mismatch expected: Warning: object is deprecated Alternative non-deprecated CRS:   EPSG:3003  PROJ.4 string: +proj=tmerc +lat_0=0 +lon_0=9 +k=0.9996 +x_0=1 | Testing deprecated CRS |
| `test_projinfo.yaml#23` | projinfo | exit 1 expected 0 GEOGCS should have a PRIMEM node  | Testing non compliant WKT1 |
| `test_projinfo.yaml#25` | projinfo | text mismatch expected: Candidate operations found: 2 ------------------------------------- Operation No. 1:  INVERSE(DERIVED_FROM(PROJ)):EPSG_4977_TO_EPSG_5613 | Testing RH2000 height to ETRS89-SWE [SWEREF 99] |
| `test_projinfo.yaml#26` | projinfo | text mismatch expected: Candidate operations found: 3 ------------------------------------- Operation No. 1:  unknown id, Inverse of NAD83(2011) to NAVD88 heigh | Testing NAD83(2011) + NAVD88 height -> NAD83(2011) |
| `test_projinfo.yaml#27` | projinfo | text mismatch expected: Candidate operations found: 2 ------------------------------------- Operation No. 1:  INVERSE(DERIVED_FROM(EPSG)):10000, Inverse of ETRS | Testing NGF IGN69 height to RGF93 |
| `test_projinfo.yaml#28` | projinfo | text mismatch expected: PROJ.4 string: +proj=utm +zone=31 +datum=WGS84 +units=m +no_defs +type=crs  WKT2:2019 string: PROJCRS["WGS 84 / UTM zone 31N",     BASEG | - |
| `test_projinfo.yaml#32` | projinfo | text mismatch expected: Candidate operations found: 3 unknown id, Inverse of UTM zone 31N + WGS 84 to EGM2008 height (1), 0.113 m, World. unknown id, Inverse of | - |
| `test_projinfo.yaml#33` | projinfo | text mismatch expected: +proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad +step +proj=stere +lat_0=90 +lon_0=0 +k=0.994 +x_0=2000000 +y_0=2000000 +e | Undocumented option: --normalize-axis-order |
| `test_projinfo.yaml#35` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=vgridshift +grids=@foo.gtx +multiplier=1   +step +proj=uni | - |
| `test_projinfo.yaml#36` | projinfo | text mismatch expected: Candidate operations found: 19 unknown id, AGD66 to WGS 84 (18) + WGS 84 to WGS 84 (G1762), 5 m, Australia - offshore including EEZ. unk | - |
| `test_projinfo.yaml#37` | projinfo | text mismatch expected: Candidate operations found: 4 unknown id, Inverse of 3-degree Gauss-Kruger zone 3 + DHDN to ETRS89 (9), 0.1 m, Germany - Baden-Wurttembe | - |
| `test_projinfo.yaml#38` | projinfo | text mismatch expected: Candidate operations found: 1 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):5656, GDA94 to AHD height (49), | - |
| `test_projinfo.yaml#39` | projinfo | text mismatch expected: Candidate operations found: 1 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):8451, GDA2020 to AHD height (1) | - |
| `test_projinfo.yaml#45` | projinfo | exit 1 expected 0 -k operation is not supported yet: coordinate operations are not described  | - |
| `test_projinfo.yaml#52` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#53` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#54` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#55` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#56` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#57` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#58` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=tinshift +file=fi_nls_ykj_etrs35fin.json  got:  | - |
| `test_projinfo.yaml#59` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#60` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#61` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#62` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#63` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=tinshift +file=fi_nls_n43_n60.json   +step +proj=tinshift +file=fi_nls_n6 | - |
| `test_projinfo.yaml#64` | projinfo | text mismatch expected: +proj=pipeline   +step +inv +proj=tinshift +file=fi_nls_ykj_etrs35fin.json   +step +inv +proj=tinshift +file=fi_nls_n60_n2000.json   +st | - |
| `test_projinfo.yaml#65` | projinfo | text mismatch expected: Candidate operations found: 2 EPSG:10894, ITRF2014 to ETRS89-DNK (1), 0.006 m, Denmark - onshore and offshore., time-dependent operation | Quick check of NKG transformations |
| `test_projinfo.yaml#68` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Inverse of UTM zone 30N + ED50 to ETRS89 (12) + UTM zone 30N, 0.2 m, Spain - mainland, Baleari | Checks that ED50 to ETRS89 (12) is in the output (superseded transformation, but replacements has unknown grid) |
| `test_projinfo.yaml#83` | projinfo | text mismatch expected: +proj=tmerc +lat_0=0 +lon_0=21 +k=0.9999 +x_0=500000 +y_0=-4000000 +ellps=bessel +towgs84=521.748,229.489,590.921,4.029,4.488,-15.521,-9 | Test North-Macedonia CRS for which there are several MGI 1901 -> WGS 84 Helmert transformations but only one matches the extent of the CRS |
| `test_projinfo.yaml#84` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Null geographic offset from NAD83(CSRS)v7 (geog2D) to NAD83(CSRS)v7 (geog3D) + Canada velocity | - |
| `test_projinfo.yaml#85` | projinfo | text mismatch expected: +proj=noop got:  | Test that "CGVD28 height" to "CGVD28(HTv2.0) height" is a no-op |
| `test_projinfo.yaml#86` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=vgridshift +grids=us_nga | Test custom records for WGS 84 realizations to EGM2008 (https://github.com/OSGeo/PROJ/issues/4362) |
| `test_projinfo.yaml#87` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=cart +ellps=WGS84   +ste | Test 'WGS 84 (Gxxx) + EGM2008 height' to 'WGS 84 (Gyyy) + EGM2008 height' |
| `test_projinfo.yaml#88` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRF2005 (geog3D) to ETRF2005 (geocentric) + Inverse of ITRF2005 to ETRF2005 ( | Test that ETRF2000 to ETRF2014 only goes through ITRF2000, ITRF2008 or ITRF2014, and not ITRF9x or ITRF>2020 |
| `test_projinfo.yaml#89` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRS89-POL [PL-ETRF2000] (geog2D) to ETRS89-POL [PL-ETRF2000] (geocentric) + I | Test that ETRF2000-PL to ETRF2014 is not negatively affected by https://github.com/OSGeo/PROJ/pull/4364 changes (the current actual result is not golden in any way, using ETRF2000 to ETRF2014 could probably make sense) |
| `test_projinfo.yaml#90` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Conversion from WGS 84 (G1150) (geog2D) to WGS 84 (G1150) (geocentric) + WGS 84 (G1150) to WGS | Test WGS 84 (G1150) to WGS 84 (G2296) |
| `test_projinfo.yaml#106` | projinfo | text mismatch expected: +proj=noop  got: Candidate operations found: 0   | - |
| `test_projinfo.yaml#108` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Inverse of Saba to WGS 84 (1) + Saba to Saba height (1) + Saba Transverse Mercator 2020, 1.2 m | Test that EPSG:4979 to EPSG:10645 does not include a non-sensical 'Inverse of Inverse of BES2020 Saba to Saba height (1) + Inverse of Saba to WGS 84 (1) + Saba Transverse Mercator 2020' |
