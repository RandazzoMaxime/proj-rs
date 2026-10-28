# Unported CLI cases

The yaml runner executes `cct` and `cs2cs` when the case is a numeric
`+proj` transform those binaries accept. Everything else is listed here.
`proj`, `geod`, `gie`, and `projsync` are not binaries of this crate.
`projinfo` is listed because WKT export is not built.

| id | app | reason | comment |
|---|---|---|---|
| `test_cct.yaml#1` | cct | case post-processes output or sets an environment | Test version argument |
| `test_cct.yaml#2` | cct | case post-processes output or sets an environment | Test help argument |
| `test_cct.yaml#10` | cct | cct invocation is not a +proj operation | Test cct with object code initialization |
| `test_cct.yaml#11` | cct | case post-processes output or sets an environment | Test cct with object that is not a CRS |
| `test_cct.yaml#12` | cct | cct invocation is not a +proj operation | Test cct with object name initialization |
| `test_cct.yaml#13` | cct | case writes files the binary does not read | Test cct with object code initialization and file input |
| `test_cct.yaml#14` | cct | case writes files the binary does not read | Test cct with WKT in a file |
| `test_cct.yaml#15` | cct | case writes files the binary does not read | Test cct with input file with UTF-8 BOM marker |
| `test_cct.yaml#16` | cct | case checks stderr text | Test cct with non existing input file |
| `test_cct.yaml#17` | cct | case expects a non-zero status our flags do not produce | Test robustness to non-ASCII characters (cf https://github.com/OSGeo/PROJ/issues/4528 case 1) |
| `test_cct.yaml#18` | cct | case expects a non-zero status our flags do not produce | Test robustness to non-ASCII characters (cf https://github.com/OSGeo/PROJ/issues/4528 case 2) |
| `test_cs2cs_various.yaml#1` | cs2cs | cs2cs input is DMS or empty | Test raw ellipse to raw ellipse |
| `test_cs2cs_various.yaml#2` | cs2cs | cs2cs input is DMS or empty | Test NAD27 to raw ellipse |
| `test_cs2cs_various.yaml#3` | cs2cs | cs2cs input is DMS or empty | Between two 3parameter approximations on same ellipsoid |
| `test_cs2cs_various.yaml#4` | cs2cs | cs2cs input is DMS or empty | 3param to raw ellipsoid on same ellipsoid |
| `test_cs2cs_various.yaml#5` | cs2cs | prime meridian or geocentric latitude is not what cs2cs prints | Test simple prime meridian handling |
| `test_cs2cs_various.yaml#6` | cs2cs | cs2cs input is DMS or empty | Test support for the lon_wrap switch |
| `test_cs2cs_various.yaml#7` | cs2cs | prime meridian or geocentric latitude is not what cs2cs prints | Test simple prime meridian handling within a projection |
| `test_cs2cs_various.yaml#8` | cs2cs | grad input is not what cs2cs reads | Test input in grad |
| `test_cs2cs_various.yaml#9` | cs2cs | cs2cs input is DMS or empty | Test geocentric x/y/z generation |
| `test_cs2cs_various.yaml#10` | cs2cs | cs2cs reads only two axes | Test geocentric x/y/z consumption |
| `test_cs2cs_various.yaml#11` | cs2cs | prime meridian or geocentric latitude is not what cs2cs prints | Test conversion from geodetic latlong to geocentric latlong |
| `test_cs2cs_various.yaml#12` | cs2cs | prime meridian or geocentric latitude is not what cs2cs prints | Test conversion from geocentric latlong to geodetic latlong |
| `test_cs2cs_various.yaml#17` | cs2cs | cs2cs input is DMS or empty | Test RSO Borneo projection (re: ticket 62) |
| `test_cs2cs_various.yaml#19` | cs2cs | cs2cs input is DMS or empty | Test extended transverse mercator inverse (#97) |
| `test_cs2cs_various.yaml#21` | cs2cs | cs2cs input is DMS or empty | Test transverse mercator inverse (#97) |
| `test_cs2cs_various.yaml#23` | cs2cs | cs2cs reads only two axes | Test robinson inverse projection (#113) |
| `test_cs2cs_various.yaml#25` | cs2cs | cs2cs reads only two axes | Test hammer inverse projection (pull request #329) |
| `test_cs2cs_various.yaml#26` | cs2cs | flag is not implemented on this binary | Test healpix forward projection on sphere |
| `test_cs2cs_various.yaml#27` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#28` | cs2cs | flag is not implemented on this binary | Test healpix forward projection on ellipsoid |
| `test_cs2cs_various.yaml#29` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#30` | cs2cs | flag is not implemented on this binary | Test healpix inverse projection on ellipsoid |
| `test_cs2cs_various.yaml#31` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#32` | cs2cs | flag is not implemented on this binary | Those points used to be in the previous test, but precision requirement reduced here to 1 decimal because of i386 failures (cf https://github.com/OSGeo/PROJ/pull/4441#issuecomment-2744141103) |
| `test_cs2cs_various.yaml#33` | cs2cs | flag is not implemented on this binary | Test rHEALPix forward projection on sphere north=0 south=0 |
| `test_cs2cs_various.yaml#34` | cs2cs | flag is not implemented on this binary | Test rHEALPix forward projection on sphere north=1 south=1 |
| `test_cs2cs_various.yaml#35` | cs2cs | flag is not implemented on this binary | Test rHEALPix inverse projection on sphere north=0 south=0 |
| `test_cs2cs_various.yaml#36` | cs2cs | flag is not implemented on this binary | Test rHEALPix inverse projection on sphere north=1 south=1 |
| `test_cs2cs_various.yaml#37` | cs2cs | flag is not implemented on this binary | Test rHEALPix forward projection on ellipsoid north=0 south=0 |
| `test_cs2cs_various.yaml#38` | cs2cs | flag is not implemented on this binary | Test rHEALPix forward projection on ellipsoid north=1 south=1 |
| `test_cs2cs_various.yaml#39` | cs2cs | flag is not implemented on this binary | Test rHEALPix inverse projection on ellipsoid north=0 south=0 |
| `test_cs2cs_various.yaml#40` | cs2cs | flag is not implemented on this binary | Test rHEALPix inverse projection on ellipsoid north=1 south=1 |
| `test_cs2cs_various.yaml#41` | cs2cs | cs2cs input is DMS or empty | Test geos on a sphere |
| `test_cs2cs_various.yaml#42` | cs2cs | cs2cs input is DMS or empty | Test geos on a ellipsoid |
| `test_cs2cs_various.yaml#43` | cs2cs | flag is not implemented on this binary | Test inv geos on a sphere |
| `test_cs2cs_various.yaml#44` | cs2cs | flag is not implemented on this binary | Test inv geos on a ellipsoid |
| `test_cs2cs_various.yaml#45` | cs2cs | cs2cs input is DMS or empty | Test geos on a sphere with alternate sweep |
| `test_cs2cs_various.yaml#46` | cs2cs | cs2cs input is DMS or empty | Test geos on a ellipsoid with alternate sweep |
| `test_cs2cs_various.yaml#47` | cs2cs | flag is not implemented on this binary | Test inv geos on a sphere with alternate sweep |
| `test_cs2cs_various.yaml#48` | cs2cs | flag is not implemented on this binary | Test inv geos on a ellipsoid with alternate sweep |
| `test_cs2cs_various.yaml#49` | cs2cs | flag is not implemented on this binary | Test the Natural Earth Projection |
| `test_cs2cs_various.yaml#50` | cs2cs | flag is not implemented on this binary | Test the Natural Earth II Projection |
| `test_cs2cs_various.yaml#51` | cs2cs | flag is not implemented on this binary | Test the Compact Miller projection |
| `test_cs2cs_various.yaml#54` | cs2cs | flag is not implemented on this binary | Test laea |
| `test_cs2cs_various.yaml#55` | cs2cs | cs2cs input is DMS or empty | Test forward calcofi projection |
| `test_cs2cs_various.yaml#57` | cs2cs | cs2cs arguments are not SOURCE +to TARGET | Check inverse error handling with ob_tran (#225) |
| `test_cs2cs_various.yaml#58` | cs2cs | flag is not implemented on this binary | Test inverse handling |
| `test_cs2cs_various.yaml#59` | cs2cs | flag is not implemented on this binary | Test MGI datum gives expected results (#207) |
| `test_cs2cs_various.yaml#60` | cs2cs | flag is not implemented on this binary | Test omerc sensitivity with locations 90d from origin (#114) |
| `test_cs2cs_various.yaml#61` | cs2cs | flag is not implemented on this binary | Test omerc differences between poles (#190) - north pole |
| `test_cs2cs_various.yaml#62` | cs2cs | flag is not implemented on this binary | Test omerc differences between poles (#190) - south pole |
| `test_cs2cs_various.yaml#63` | cs2cs | flag is not implemented on this binary | Test qsc |
| `test_cs2cs_various.yaml#64` | cs2cs | flag is not implemented on this binary | Test inv qsc |
| `test_cs2cs_various.yaml#65` | cs2cs | flag is not implemented on this binary | Test bug 229 |
| `test_cs2cs_various.yaml#66` | cs2cs | flag is not implemented on this binary | Test bug 229 (2) |
| `test_cs2cs_various.yaml#67` | cs2cs | flag is not implemented on this binary | Test bug 244 |
| `test_cs2cs_various.yaml#68` | cs2cs | flag is not implemented on this binary | Test bug 244 (2) |
| `test_cs2cs_various.yaml#69` | cs2cs | flag is not implemented on this binary | Test bug 245 (use +datum=carthage) |
| `test_cs2cs_various.yaml#70` | cs2cs | flag is not implemented on this binary | Test bug 245 (use expansion of +datum=carthage) |
| `test_cs2cs_various.yaml#71` | cs2cs | flag is not implemented on this binary | Test SCH forward projection |
| `test_cs2cs_various.yaml#72` | cs2cs | flag is not implemented on this binary | Test SCH inverse projection |
| `test_cs2cs_various.yaml#73` | cs2cs | flag is not implemented on this binary | Test issue #316 (switch utm to use etmerc) |
| `test_cs2cs_various.yaml#74` | cs2cs | flag is not implemented on this binary | Test issue #316 (switch utm to use etmerc) |
| `test_cs2cs_various.yaml#75` | cs2cs | flag is not implemented on this binary | Test nzmg forward projection |
| `test_cs2cs_various.yaml#76` | cs2cs | flag is not implemented on this binary | Test nzmg inverse projection |
| `test_cs2cs_various.yaml#77` | cs2cs | flag is not implemented on this binary | Test patterson forward projection |
| `test_cs2cs_various.yaml#78` | cs2cs | flag is not implemented on this binary | Test patterson inverse projection |
| `test_cs2cs_various.yaml#79` | cs2cs | flag is not implemented on this binary | Test Web Mercator to avoid issue #834 in the future |
| `test_cs2cs_various.yaml#80` | cs2cs | flag is not implemented on this binary | Test vto_meter |
| `test_cs2cs_various.yaml#81` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#82` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#83` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#84` | cs2cs | cs2cs reads only two axes | Test EPSG:4326 to EPSG:32631 |
| `test_cs2cs_various.yaml#85` | cs2cs | cs2cs reads only two axes | Test EPSG:32631 to EPSG:4326 |
| `test_cs2cs_various.yaml#86` | cs2cs | authority other than an EPSG code | Check we allow ellipsoids up to a 0.59% relative difference in size |
| `test_cs2cs_various.yaml#87` | cs2cs | flag is not implemented on this binary | Test EPSG:4896 to EPSG:7930 |
| `test_cs2cs_various.yaml#88` | cs2cs | flag is not implemented on this binary | Test ITRF2000 to ITRF1993 |
| `test_cs2cs_various.yaml#89` | cs2cs | flag is not implemented on this binary | Check ob_tran with o_proj=longlat (#1525) |
| `test_cs2cs_various.yaml#90` | cs2cs | flag is not implemented on this binary | Test inverse handling |
| `test_cs2cs_various.yaml#91` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#92` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#93` | cs2cs | case writes files the binary does not read | Test EPSG:xxxx EPSG:yyyy filename |
| `test_cs2cs_various.yaml#94` | cs2cs | flag is not implemented on this binary | Test Colombia Urban |
| `test_cs2cs_various.yaml#95` | cs2cs | authority other than an EPSG code | Test effect of --authority (GH-2442) - use the EPSG:8076 operation |
| `test_cs2cs_various.yaml#96` | cs2cs | flag is not implemented on this binary | Test effect of --authority (GH-2442) - a no-op |
| `test_cs2cs_various.yaml#97` | cs2cs | case post-processes output or sets an environment | Test effect of --accuracy |
| `test_cs2cs_various.yaml#98` | cs2cs | case post-processes output or sets an environment | Test effect of --no-ballpark |
| `test_cs2cs_various.yaml#99` | cs2cs | argument is a file reference or a quoted CRS name | Check that we can use a transformation spanning the antimeridian (should use Pulkovo 1942 to WGS 84 (20)) |
| `test_cs2cs_various.yaml#100` | cs2cs | argument is a file reference or a quoted CRS name | - |
| `test_cs2cs_various.yaml#101` | cs2cs | argument is a file reference or a quoted CRS name | Check that we select the operation that has the smallest area of use, when 2 have the same accuracy |
| `test_cs2cs_various.yaml#102` | cs2cs | case post-processes output or sets an environment | Check that we promote CRS specified by name to 3D when the other one is 3D |
| `test_cs2cs_various.yaml#103` | cs2cs | case post-processes output or sets an environment | - |
| `test_cs2cs_various.yaml#104` | cs2cs | case writes files the binary does not read | Test input file with UTF-8 BOM marker |
| `test_cs2cs_various.yaml#105` | cs2cs | flag is not implemented on this binary | Test cs2cs -W0 |
| `test_cs2cs_various.yaml#106` | cs2cs | flag is not implemented on this binary | Test cs2cs -W8 |
| `test_cs2cs_various.yaml#107` | cs2cs | case post-processes output or sets an environment | Test cs2cs -W |
| `test_cs2cs_various.yaml#108` | cs2cs | case post-processes output or sets an environment | Test cs2cs -W9 |
| `test_cs2cs_various.yaml#109` | cs2cs | case post-processes output or sets an environment | Test cs2cs -W10 |
| `test_cs2cs_various.yaml#110` | cs2cs | flag is not implemented on this binary | Test cs2cs --only-best (grid missing) for NTF to RGF93 |
| `test_cs2cs_various.yaml#111` | cs2cs | case post-processes output or sets an environment | Test cs2cs --only-best (working) for EPSG:4326+5773 to EPSG:4979 |
| `test_cs2cs_various.yaml#112` | cs2cs | flag is not implemented on this binary | Test cs2cs --only-best (working) for EPSG:4326 to GDA94 |
| `test_cs2cs_various.yaml#113` | cs2cs | flag is not implemented on this binary | Test cs2cs --only-best (working) for GDA94 to EPSG:4326 |
| `test_cs2cs_various.yaml#114` | cs2cs | flag is not implemented on this binary | Test cs2cs --only-best (working) for EPSG:4326 to GDA2020 |
| `test_cs2cs_various.yaml#115` | cs2cs | flag is not implemented on this binary | Test cs2cs --only-best (working) for GDA2020 to EPSG:4326 |
| `test_cs2cs_various.yaml#116` | cs2cs | case post-processes output or sets an environment | Test cs2cs (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#117` | cs2cs | case post-processes output or sets an environment | Test cs2cs --only-best=no (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#118` | cs2cs | case post-processes output or sets an environment | Test cs2cs --only-best (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#119` | cs2cs | case post-processes output or sets an environment | Test cs2cs --only-best=yes (grid missing) for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#120` | cs2cs | case post-processes output or sets an environment | Test cs2cs (grid missing) with PROJ_ONLY_BEST_DEFAULT=YES for EPSG:4326+3855 to EPSG:4979 |
| `test_cs2cs_various.yaml#121` | cs2cs | case writes files the binary does not read | Test cs2cs (grid missing) with only_best_default=on in proj.ini |
| `test_cs2cs_various.yaml#122` | cs2cs | case post-processes output or sets an environment | Test cs2cs --only-best (grid missing) |
| `test_cs2cs_various.yaml#123` | cs2cs | case post-processes output or sets an environment | Test cs2cs --only-best --no-ballpark (grid missing) |
| `test_cs2cs_various.yaml#124` | cs2cs | case writes files the binary does not read | Test cs2cs (grid missing) with scenario of GH-3607 that we are using the 'NAD27 to WGS 84 (6)' DMA-ConusW transformation |
| `test_cs2cs_various.yaml#125` | cs2cs | case writes files the binary does not read | Test cs2cs (grid missing) with scenario of GH-3613 that we are using a WGS 84 intermediate to do NAD27 to NAD83 |
| `test_cs2cs_various.yaml#126` | cs2cs | case writes files the binary does not read | Test cs2cs (grid missing) with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#127` | cs2cs | case writes files the binary does not read | Test cs2cs (grid missing) --only-best=no with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#128` | cs2cs | case writes files the binary does not read | Test cs2cs (grid missing) --only-best with scenario of https://lists.osgeo.org/pipermail/proj/2023-April/011003.html |
| `test_cs2cs_various.yaml#129` | cs2cs | cs2cs reads only two axes | Test cs2cs grid missing with @gridname |
| `test_cs2cs_various.yaml#130` | cs2cs | cs2cs reads only two axes | Test cs2cs geographic -> geocentric using BETA2007.gsb grid |
| `test_cs2cs_various.yaml#131` | cs2cs | cs2cs reads only two axes | Test cs2cs geocentric -> geographic using BETA2007.gsb grid |
| `test_cs2cs_various.yaml#132` | cs2cs | case writes files the binary does not read | Test cs2cs geographic -> geocentric, normally using BETA2007.gsb grid but missing |
| `test_cs2cs_various.yaml#133` | cs2cs | case writes files the binary does not read | Test cs2cs geocentric -> geographic, normally using BETA2007.gsb grid but missing |
| `test_cs2cs_various.yaml#134` | cs2cs | cs2cs reads only two axes | Test cs2cs geographic -> geocentric outside BETA2007.gsb grid |
| `test_cs2cs_various.yaml#135` | cs2cs | cs2cs reads only two axes | Test cs2cs geocentric -> geographic outside BETA2007.gsb grid |
| `test_cs2cs_various.yaml#136` | cs2cs | flag is not implemented on this binary | Test Similarity Transformation (example from EPSG Guidance Note 7.2) |
| `test_cs2cs_various.yaml#137` | cs2cs | flag is not implemented on this binary | Test inverse of Similarity Transformation (example from EPSG Guidance Note 7.2) |
| `test_cs2cs_various.yaml#138` | cs2cs | flag is not implemented on this binary | Test Similarity Transformation through CompoundCRS |
| `test_cs2cs_various.yaml#139` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#140` | cs2cs | flag is not implemented on this binary | Test --s_epoch |
| `test_cs2cs_various.yaml#141` | cs2cs | flag is not implemented on this binary | Test --t_epoch |
| `test_cs2cs_various.yaml#142` | cs2cs | flag is not implemented on this binary | - |
| `test_cs2cs_various.yaml#143` | cs2cs | case post-processes output or sets an environment | Test cs2cs EPSG:5488 (RGAF09) to EPSG:4559+5757 (RRAF 1991 / UTM zone 20N + Guadeloupe 1988 height) |
| `test_projinfo.yaml#1` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#2` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#3` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#4` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#5` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#6` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#7` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#8` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#9` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#10` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#11` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#12` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#13` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#14` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#15` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#16` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#17` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#18` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#19` | projinfo | projinfo is not built; WKT export is not built | several match |
| `test_projinfo.yaml#20` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#21` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#22` | projinfo | projinfo is not built; WKT export is not built | Testing deprecated CRS |
| `test_projinfo.yaml#23` | projinfo | projinfo is not built; WKT export is not built | Testing non compliant WKT1 |
| `test_projinfo.yaml#24` | projinfo | projinfo is not built; WKT export is not built | Testing CRS with towgs84 |
| `test_projinfo.yaml#25` | projinfo | projinfo is not built; WKT export is not built | Testing RH2000 height to ETRS89-SWE [SWEREF 99] |
| `test_projinfo.yaml#26` | projinfo | projinfo is not built; WKT export is not built | Testing NAD83(2011) + NAVD88 height -> NAD83(2011) |
| `test_projinfo.yaml#27` | projinfo | projinfo is not built; WKT export is not built | Testing NGF IGN69 height to RGF93 |
| `test_projinfo.yaml#28` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#29` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#30` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#31` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#32` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#33` | projinfo | projinfo is not built; WKT export is not built | Undocumented option: --normalize-axis-order |
| `test_projinfo.yaml#34` | projinfo | projinfo is not built; WKT export is not built | WGS 84 to ETRS89 (2) uses a transformation method not supported by PROJ currently (time-specific Helmert), and thus must be sorted last |
| `test_projinfo.yaml#35` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#36` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#37` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#38` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#39` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#40` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#41` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#42` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#43` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#44` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#45` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#46` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#47` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#48` | projinfo | projinfo is not built; WKT export is not built | Hard to test content |
| `test_projinfo.yaml#49` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#50` | projinfo | projinfo is not built; WKT export is not built | Hard to test content |
| `test_projinfo.yaml#51` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#52` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#53` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#54` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#55` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#56` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#57` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#58` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#59` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#60` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#61` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#62` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#63` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#64` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#65` | projinfo | projinfo is not built; WKT export is not built | Quick check of NKG transformations |
| `test_projinfo.yaml#66` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#67` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#68` | projinfo | projinfo is not built; WKT export is not built | Checks that ED50 to ETRS89 (12) is in the output (superseded transformation, but replacements has unknown grid) |
| `test_projinfo.yaml#69` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#70` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#71` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#72` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#73` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#74` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#75` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#76` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#77` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#78` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#79` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#80` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#81` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#82` | projinfo | projinfo is not built; WKT export is not built | Test case where --area has several matches |
| `test_projinfo.yaml#83` | projinfo | projinfo is not built; WKT export is not built | Test North-Macedonia CRS for which there are several MGI 1901 -> WGS 84 Helmert transformations but only one matches the extent of the CRS |
| `test_projinfo.yaml#84` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#85` | projinfo | projinfo is not built; WKT export is not built | Test that "CGVD28 height" to "CGVD28(HTv2.0) height" is a no-op |
| `test_projinfo.yaml#86` | projinfo | projinfo is not built; WKT export is not built | Test custom records for WGS 84 realizations to EGM2008 (https://github.com/OSGeo/PROJ/issues/4362) |
| `test_projinfo.yaml#87` | projinfo | projinfo is not built; WKT export is not built | Test 'WGS 84 (Gxxx) + EGM2008 height' to 'WGS 84 (Gyyy) + EGM2008 height' |
| `test_projinfo.yaml#88` | projinfo | projinfo is not built; WKT export is not built | Test that ETRF2000 to ETRF2014 only goes through ITRF2000, ITRF2008 or ITRF2014, and not ITRF9x or ITRF>2020 |
| `test_projinfo.yaml#89` | projinfo | projinfo is not built; WKT export is not built | Test that ETRF2000-PL to ETRF2014 is not negatively affected by https://github.com/OSGeo/PROJ/pull/4364 changes (the current actual result is not golden in any way, using ETRF2000 to ETRF2014 could probably make sense) |
| `test_projinfo.yaml#90` | projinfo | projinfo is not built; WKT export is not built | Test WGS 84 (G1150) to WGS 84 (G2296) |
| `test_projinfo.yaml#91` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#92` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#93` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#94` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#95` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#96` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#97` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#98` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#99` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#100` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#101` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#102` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#103` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#104` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#105` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#106` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_projinfo.yaml#107` | projinfo | projinfo is not built; WKT export is not built | Test bugfix of https://github.com/OSGeo/PROJ/issues/4464 (NAD83(CSRS) to NAD83(CSRS)v2 should not go through NAD83) |
| `test_projinfo.yaml#108` | projinfo | projinfo is not built; WKT export is not built | Test that EPSG:4979 to EPSG:10645 does not include a non-sensical 'Inverse of Inverse of BES2020 Saba to Saba height (1) + Inverse of Saba to WGS 84 (1) + Saba Transverse Mercator 2020' |
| `test_projinfo.yaml#109` | projinfo | projinfo is not built; WKT export is not built | Test that 'projinfo EGM2008' by default resolves to the datum |
| `test_projinfo.yaml#110` | projinfo | projinfo is not built; WKT export is not built | Test that 'projinfo -k crs EGM2008' resolves to the CRS |
| `test_projinfo.yaml#111` | projinfo | projinfo is not built; WKT export is not built | - |
| `test_proj.yaml#1` | proj | executable is not built | - |
| `test_proj.yaml#2` | proj | executable is not built | Test CRS option |
| `test_proj.yaml#3` | proj | executable is not built | Test projection factors on projected CRS with non-Greenwhich prime meridian |
| `test_proj.yaml#4` | proj | executable is not built | Test projection factors on compound CRS with a projected CRS |
