# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, and `proj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_projinfo.yaml#52` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#53` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#54` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#55` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#56` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#57` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#62` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=tmerc +lat_0=0 +lon_0=27 | - |
| `test_projinfo.yaml#63` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=tinshift +file=fi_nls_n43_n60.json   +step +proj=tinshift +file=fi_nls_n6 | - |
| `test_projinfo.yaml#64` | projinfo | text mismatch expected: +proj=pipeline   +step +inv +proj=tinshift +file=fi_nls_ykj_etrs35fin.json   +step +inv +proj=tinshift +file=fi_nls_n60_n2000.json   +st | - |
| `test_projinfo.yaml#88` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRF2005 (geog3D) to ETRF2005 (geocentric) + Inverse of ITRF2005 to ETRF2005 ( | Test that ETRF2000 to ETRF2014 only goes through ITRF2000, ITRF2008 or ITRF2014, and not ITRF9x or ITRF>2020 |
| `test_projinfo.yaml#89` | projinfo | text mismatch expected: Candidate operations found: 2 unknown id, Conversion from ETRS89-POL [PL-ETRF2000] (geog2D) to ETRS89-POL [PL-ETRF2000] (geocentric) + I | Test that ETRF2000-PL to ETRF2014 is not negatively affected by https://github.com/OSGeo/PROJ/pull/4364 changes (the current actual result is not golden in any way, using ETRF2000 to ETRF2014 could probably make sense) |
