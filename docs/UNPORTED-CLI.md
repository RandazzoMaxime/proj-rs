# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, and `proj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_cs2cs_various.yaml#9` | cs2cs | expected 6378137.00	-0.03 0.03 got 6378137.00	-0.03 0.00 | Test geocentric x/y/z generation |
| `test_cs2cs_various.yaml#10` | cs2cs | expected 0dE	0dN 10.000 got 0dE	0dN 0.000 | Test geocentric x/y/z consumption |
| `test_cs2cs_various.yaml#71` | cs2cs | expected -1977112.0305592	5551475.1418378 6595.7256583 got -1977112.0305592	5551475.1418378 0.0000000 | Test SCH forward projection |
| `test_cs2cs_various.yaml#72` | cs2cs | expected 44.989863	29.998124 -0.000362 got 44.989863	29.998124 0.000000 | Test SCH inverse projection |
| `test_cs2cs_various.yaml#80` | cs2cs | expected 0.000	0.000 1000.000 got 0.000	0.000 1.000 | Test vto_meter |
| `test_cs2cs_various.yaml#81` | cs2cs | expected 0.000	0.000 1000.000 got 0.000	0.000 1.000 | - |
| `test_cs2cs_various.yaml#96` | cs2cs | expected 49dN	2dE 0.000 got 49d0'0.002"N	2dE 0.018 | Test effect of --authority (GH-2442) - a no-op |
| `test_cs2cs_various.yaml#102` | cs2cs | expected 49.000	2.000 4.936 got 49.000	2.000 50.000 | Check that we promote CRS specified by name to 3D when the other one is 3D |
| `test_cs2cs_various.yaml#103` | cs2cs | expected 49.000	2.000 45.064 got 49.000	2.000 0.000 | - |
| `test_cs2cs_various.yaml#120` | cs2cs | no numbers expected *	* inf got 49dN	2dE 0.000 | Test cs2cs (grid missing) with PROJ_ONLY_BEST_DEFAULT=YES for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#121` | cs2cs | no numbers expected *	* inf got 49dN	2dE 0.000 | Test cs2cs (grid missing) with only_best_default=on in proj.ini |
| `test_cs2cs_various.yaml#124` | cs2cs | expected 1402288.54	5076296.64 0.00 got 1402224.57	5076275.42 0.00 | Test cs2cs (grid missing) with scenario of GH-3607 that we are using the 'NAD27 to WGS 84 (6)' DMA-ConusW transformation |
| `test_cs2cs_various.yaml#125` | cs2cs | expected 569720.46	4268813.88 0.00 got 569706.33	4268817.68 0.00 | Test cs2cs (grid missing) with scenario of GH-3613 that we are using a WGS 84 intermediate to do NAD27 to NAD83 |
| `test_cs2cs_various.yaml#126` | cs2cs | stderr text mismatch expected: Attempt to use coordinate operation Inverse of WGS 84 to EGM96 height (1) + ETRS89-ESP [REGENTE] to Alicante height (1) using ETR | Test cs2cs (grid missing) with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#128` | cs2cs | exit 0 expected 3  | Test cs2cs (grid missing) --only-best with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#129` | cs2cs | expected 49dN	2dE 0.000 got 2dE	49dN 0.000 | Test cs2cs grid missing with @gridname |
| `test_cs2cs_various.yaml#132` | cs2cs | expected 4003461.37	705832.04 4898267.94 got 4003345.54	705897.83 4898352.56 | Test cs2cs geographic -> geocentric, normally using BETA2007.gsb grid but missing |
| `test_cs2cs_various.yaml#133` | cs2cs | expected 50d30'N	10dE -0.002 got 50d29'55.695"N	9d59'55.692"E -0.002 | Test cs2cs geocentric -> geographic, normally using BETA2007.gsb grid but missing |
| `test_cs2cs_various.yaml#139` | cs2cs | expected 476763.303	95620.222 5.000 got 477131.509	95651.636 5.000 | - |
| `test_projinfo.yaml#13` | projinfo | text mismatch expected: Candidate operations found: 9 DERIVED_FROM(EPSG):1313, NAD27 to NAD83 (4), 1.5 m, Canada - onshore - Alberta; British Columbia; Manitoba | - |
| `test_projinfo.yaml#14` | projinfo | text mismatch expected: Candidate operations found: 10 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):1313, NAD27 to NAD83 (4), 1.5  | - |
| `test_projinfo.yaml#23` | projinfo | exit 1 expected 0 GEOGCS should have a PRIMEM node  | Testing non compliant WKT1 |
| `test_projinfo.yaml#25` | projinfo | text mismatch expected: Candidate operations found: 2 ------------------------------------- Operation No. 1:  INVERSE(DERIVED_FROM(PROJ)):EPSG_4977_TO_EPSG_5613 | Testing RH2000 height to ETRS89-SWE [SWEREF 99] |
| `test_projinfo.yaml#26` | projinfo | text mismatch expected: Candidate operations found: 3 ------------------------------------- Operation No. 1:  unknown id, Inverse of NAD83(2011) to NAVD88 heigh | Testing NAD83(2011) + NAVD88 height -> NAD83(2011) |
| `test_projinfo.yaml#27` | projinfo | text mismatch expected: Candidate operations found: 2 ------------------------------------- Operation No. 1:  INVERSE(DERIVED_FROM(EPSG)):10000, Inverse of ETRS | Testing NGF IGN69 height to RGF93 |
| `test_projinfo.yaml#28` | projinfo | text mismatch expected: PROJ.4 string: +proj=utm +zone=31 +datum=WGS84 +units=m +no_defs +type=crs  WKT2:2019 string: PROJCRS["WGS 84 / UTM zone 31N",     BASEG | - |
| `test_projinfo.yaml#29` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, WGS 84 to EGM96 height (1), 1 m, World.  got: Candidate operations found: 1 unknown id, WGS 84 | - |
| `test_projinfo.yaml#30` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Inverse of WGS 84 to EGM96 height (1), 1 m, World.  got: Candidate operations found: 1 unknown | - |
| `test_projinfo.yaml#32` | projinfo | text mismatch expected: Candidate operations found: 3 unknown id, Inverse of UTM zone 31N + WGS 84 to EGM2008 height (1), 0.113 m, World. unknown id, Inverse of | - |
| `test_projinfo.yaml#36` | projinfo | text mismatch expected: Candidate operations found: 19 unknown id, AGD66 to WGS 84 (18) + WGS 84 to WGS 84 (G1762), 5 m, Australia - offshore including EEZ. unk | - |
| `test_projinfo.yaml#37` | projinfo | text mismatch expected: Candidate operations found: 4 unknown id, Inverse of 3-degree Gauss-Kruger zone 3 + DHDN to ETRS89 (9), 0.1 m, Germany - Baden-Wurttembe | - |
| `test_projinfo.yaml#38` | projinfo | text mismatch expected: Candidate operations found: 1 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):5656, GDA94 to AHD height (49), | - |
| `test_projinfo.yaml#39` | projinfo | text mismatch expected: Candidate operations found: 1 ------------------------------------- Operation No. 1:  DERIVED_FROM(EPSG):8451, GDA2020 to AHD height (1) | - |
| `test_projinfo.yaml#52` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#53` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#54` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#55` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#56` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#57` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
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
| `test_projinfo.yaml#86` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=vgridshift +grids=us_nga | Test custom records for WGS 84 realizations to EGM2008 (https://github.com/OSGeo/PROJ/issues/4362) |
| `test_projinfo.yaml#88` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRF2005 (geog3D) to ETRF2005 (geocentric) + Inverse of ITRF2005 to ETRF2005 ( | Test that ETRF2000 to ETRF2014 only goes through ITRF2000, ITRF2008 or ITRF2014, and not ITRF9x or ITRF>2020 |
| `test_projinfo.yaml#89` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRS89-POL [PL-ETRF2000] (geog2D) to ETRS89-POL [PL-ETRF2000] (geocentric) + I | Test that ETRF2000-PL to ETRF2014 is not negatively affected by https://github.com/OSGeo/PROJ/pull/4364 changes (the current actual result is not golden in any way, using ETRF2000 to ETRF2014 could probably make sense) |
| `test_projinfo.yaml#90` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Conversion from WGS 84 (G1150) (geog2D) to WGS 84 (G1150) (geocentric) + WGS 84 (G1150) to WGS | Test WGS 84 (G1150) to WGS 84 (G2296) |
| `test_projinfo.yaml#106` | projinfo | text mismatch expected: +proj=noop  got: Candidate operations found: 0   | - |
| `test_projinfo.yaml#108` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Inverse of Saba to WGS 84 (1) + Saba to Saba height (1) + Saba Transverse Mercator 2020, 1.2 m | Test that EPSG:4979 to EPSG:10645 does not include a non-sensical 'Inverse of Inverse of BES2020 Saba to Saba height (1) + Inverse of Saba to WGS 84 (1) + Saba Transverse Mercator 2020' |
