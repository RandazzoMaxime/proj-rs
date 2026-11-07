# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, and `proj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_projinfo.yaml#53` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#54` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#55` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#56` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
| `test_projinfo.yaml#57` | projinfo | text mismatch expected: +proj=pipeline   +step +proj=axisswap +order=2,1   +step +proj=unitconvert +xy_in=deg +xy_out=rad   +step +proj=defmodel +model=nz_linz_ | - |
