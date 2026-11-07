# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, and `proj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_projinfo.yaml#36` | projinfo | text mismatch expected: Candidate operations found: 19 unknown id, AGD66 to WGS 84 (18) + WGS 84 to WGS 84 (G1762), 5 m, Australia - offshore including EEZ. unk | - |
| `test_projinfo.yaml#52` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#53` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#54` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#55` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#56` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#57` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#62` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#63` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=tinshift +file=fi_nls_n43_n60.json   +step +proj=tinshift +file=fi_nls_n6 | - |
| `test_projinfo.yaml#64` | projinfo | text mismatch expected: +proj=pipeline   +step +inv +proj=tinshift +file=fi_nls_ykj_etrs35fin.json   +step +inv +proj=tinshift +file=fi_nls_n60_n2000.json   +st | - |
| `test_projinfo.yaml#65` | projinfo | text mismatch expected: Candidate operations found: 2 EPSG:10894, ITRF2014 to ETRS89-DNK (1), 0.006 m, Denmark - onshore and offshore., time-dependent operation | Quick check of NKG transformations |
| `test_projinfo.yaml#84` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Null geographic offset from NAD83(CSRS)v7 (geog2D) to NAD83(CSRS)v7 (geog3D) + Canada velocity | - |
| `test_projinfo.yaml#88` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRF2005 (geog3D) to ETRF2005 (geocentric) + Inverse of ITRF2005 to ETRF2005 ( | Test that ETRF2000 to ETRF2014 only goes through ITRF2000, ITRF2008 or ITRF2014, and not ITRF9x or ITRF>2020 |
| `test_projinfo.yaml#89` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRS89-POL [PL-ETRF2000] (geog2D) to ETRS89-POL [PL-ETRF2000] (geocentric) + I | Test that ETRF2000-PL to ETRF2014 is not negatively affected by https://github.com/OSGeo/PROJ/pull/4364 changes (the current actual result is not golden in any way, using ETRF2000 to ETRF2014 could probably make sense) |
| `test_projinfo.yaml#90` | projinfo | text mismatch expected: Candidate operations found: 1 unknown id, Conversion from WGS 84 (G1150) (geog2D) to WGS 84 (G1150) (geocentric) + WGS 84 (G1150) to WGS | Test WGS 84 (G1150) to WGS 84 (G2296) |
| `test_projinfo.yaml#106` | projinfo | text mismatch expected: +proj=noop  got: Candidate operations found: 0  | - |
