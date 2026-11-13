# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, `proj`, and `invproj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_proj_nad83.yaml#1` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#2` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#3` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#4` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#5` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#6` | proj | text mismatch expected: 1640416.667	916074.824   1640416.667    916074.825 AK 2 GP1 got: -7420640.779	42894761.582   1640416.667    916074.825 AK 2 GP1  | - |
| `test_proj_nad83.yaml#7` | proj | text mismatch expected: 1640416.667	916074.824   1640416.667    916074.825 AK 3 GP1 got: -6690244.171	43337546.818   1640416.667    916074.825 AK 3 GP1  | - |
| `test_proj_nad83.yaml#8` | proj | text mismatch expected: 1640416.667	916074.824   1640416.667    916074.825 AK 4 GP1 got: -5941058.251	43728143.223   1640416.667    916074.825 AK 4 GP1  | - |
| `test_proj_nad83.yaml#9` | proj | text mismatch expected: 1910718.662	2520810.679   1910718.662   2520810.680 AK 5 GP1 got: -4797226.284	42437102.812   1910718.662   2520810.680 AK 5 GP1  | - |
| `test_proj_nad83.yaml#10` | proj | text mismatch expected: 1640416.667	916074.824   1640416.667    916074.825 AK 5 GP2 got: -5176393.615	44067356.874   1640416.667    916074.825 AK 5 GP2  | - |
| `test_proj_nad83.yaml#11` | proj | text mismatch expected: 1998036.998	6224208.215   1998036.998   6224208.217 AK 6 GP1 got: -2906267.722	39163556.228   1998036.998   6224208.217 AK 6 GP1  | - |
| `test_proj_nad83.yaml#12` | proj | text mismatch expected: 1640416.667	6215353.365   1640416.667   6215353.367 AK 6 GP1 got: -2572566.742	39300756.879   1640416.667   6215353.367 AK 6 GP1  | - |
| `test_proj_nad83.yaml#13` | proj | text mismatch expected: 1640416.667	4111446.440   1640416.667   4111446.441 AK 7 GP1 got: -2730019.060	41485104.021   1640416.667   4111446.441 AK 7 GP1  | - |
| `test_proj_nad83.yaml#14` | proj | text mismatch expected: 1640416.667	4111446.440   1640416.667   4111446.441 AK 8 GP1 got: -2132598.196	41640700.888   1640416.667   4111446.441 AK 8 GP1  | - |
| `test_proj_nad83.yaml#15` | proj | text mismatch expected: 1640416.667	3410489.716   1640416.667   3410489.717 AK 9 GP1 got: -1638499.854	42451707.453   1640416.667   3410489.717 AK 9 GP1  | - |
| `test_proj_nad83.yaml#16` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#17` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#18` | proj | text mismatch expected: 656166.667	303104.183    656166.667    303104.183 AL E GP1 got: -26397686.654	30342587.474    656166.667    303104.183 AL E GP1  | - |
| `test_proj_nad83.yaml#19` | proj | text mismatch expected: 847539.085	782420.807    847539.085    782420.807 AL E GP2 got: -25463472.193	30116115.767    847539.085    782420.807 AL E GP2  | - |
| `test_proj_nad83.yaml#20` | proj | text mismatch expected: 421151.975	1571298.908    421151.975   1571298.908 AL E GP3 got: -24166179.816	31049179.283    421151.975   1571298.908 AL E GP3  | - |
| `test_proj_nad83.yaml#21` | proj | text mismatch expected: 1968500.000	1212527.586   1968500.000   1212527.587 AL W GP1 got: -25161013.759	31434956.109   1968500.000   1212527.587 AL W GP1  | - |
| `test_proj_nad83.yaml#22` | proj | text mismatch expected: 1968500.000	1215559.708   1968500.000   1215559.708 AL W GP2 got: -25155553.923	31435385.307   1968500.000   1215559.708 AL W GP2  | - |
| `test_proj_nad83.yaml#23` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#24` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#25` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#26` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#27` | proj | text mismatch expected: 699998.600	1364309.666    699998.600   1364309.666 AZ C GP1 got: -20971562.416	43130095.302    699998.600   1364309.666 AZ C GP1  | - |
| `test_proj_nad83.yaml#28` | proj | text mismatch expected: 699998.600	484994.340    699998.600    484994.340 AZ C GP2 got: -22094073.668	43942230.242    699998.600    484994.340 AZ C GP2  | - |
| `test_proj_nad83.yaml#29` | proj | text mismatch expected: 625301.460	1515899.830    625301.460   1515899.830 AZ E GP1 got: -21164818.971	42421392.424    625301.460   1515899.830 AZ E GP1  | - |
| `test_proj_nad83.yaml#30` | proj | text mismatch expected: 883142.524	363539.663    883142.524    363539.663 AZ E GP2 got: -22939335.134	43089737.481    883142.524    363539.663 AZ E GP2  | - |
| `test_proj_nad83.yaml#31` | proj | text mismatch expected: 597423.277	1500785.234    597423.277   1500785.235 AZ E GP3 got: -21161252.106	42470542.108    597423.277   1500785.235 AZ E GP3  | - |
| `test_proj_nad83.yaml#32` | proj | text mismatch expected: 827818.955	416705.394    827818.955    416705.394 AZ E GP4 got: -22815074.069	43116500.268    827818.955    416705.394 AZ E GP4  | - |
| `test_proj_nad83.yaml#33` | proj | text mismatch expected: 699998.600	1364355.147    699998.600   1364355.147 AZ W GP1 got: -20473081.954	43809211.882    699998.600   1364355.147 AZ W GP1  | - |
| `test_proj_nad83.yaml#34` | proj | text mismatch expected: 699998.600	1367387.968    699998.600   1367387.968 AZ W GP2 got: -20469434.151	43806412.766    699998.600   1367387.968 AZ W GP2  | - |
| `test_proj_nad83.yaml#35` | proj | text mismatch expected: 699998.600	1370420.793    699998.600   1370420.793 AZ W GP3 got: -20465786.545	43803614.376    699998.600   1370420.793 AZ W GP3  | - |
| `test_proj_nad83.yaml#36` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#37` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#38` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#39` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#40` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#41` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#42` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#43` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#44` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#45` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#46` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#47` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#48` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#49` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#50` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#51` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#52` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#53` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#54` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#55` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#56` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#57` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#58` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#59` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#60` | proj | text mismatch expected: 618403.524	493238.843    618403.524    493238.843 DE   GP1 got: -20313881.711	26639762.506    618403.524    493238.843 DE   GP1  | - |
| `test_proj_nad83.yaml#61` | proj | text mismatch expected: 684135.532	638883.528    684135.532    638883.528 DE   GP2 got: -20074439.762	26630641.384    684135.532    638883.528 DE   GP2  | - |
| `test_proj_nad83.yaml#62` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#63` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#64` | proj | text mismatch expected: 875449.222	958850.568    875449.222    958850.568 GA E GP1 got: -25034301.286	28062977.900    875449.222    958850.568 GA E GP1  | - |
| `test_proj_nad83.yaml#65` | proj | text mismatch expected: 322535.391	1274748.301    322535.391   1274748.301 GA E GP2 got: -24721646.138	29157220.155    322535.391   1274748.301 GA E GP2  | - |
| `test_proj_nad83.yaml#66` | proj | text mismatch expected: 1472470.137	287083.198   1472470.137    287083.198 HI 1 GP1 got: -8456704.378	57887021.584   1472470.137    287083.198 HI 1 GP1  | - |
| `test_proj_nad83.yaml#67` | proj | text mismatch expected: 1708685.701	250676.240   1708685.701    250676.240 HI 1 GP2 got: -8716230.966	57887387.007   1708685.701    250676.240 HI 1 GP2  | - |
| `test_proj_nad83.yaml#68` | proj | text mismatch expected: 1640416.667	314739.275   1640416.667    314739.275 HI 1 GP3 got: -8632280.275	57829759.481   1640416.667    314739.275 HI 1 GP3  | - |
| `test_proj_nad83.yaml#69` | proj | text mismatch expected: 1640416.667	317765.760   1640416.667    317765.760 HI 1 GP4 got: -8631780.216	57826508.553   1640416.667    317765.760 HI 1 GP4  | - |
| `test_proj_nad83.yaml#70` | proj | text mismatch expected: 1640416.667	133177.588   1640416.667    133177.588 HI 2 GP1 got: -8144954.319	57506912.297   1640416.667    133177.588 HI 2 GP1  | - |
| `test_proj_nad83.yaml#71` | proj | text mismatch expected: 1640416.667	136204.417   1640416.667    136204.417 HI 2 GP2 got: -8144462.340	57503689.316   1640416.667    136204.417 HI 2 GP2  | - |
| `test_proj_nad83.yaml#72` | proj | text mismatch expected: 1640416.667	121084.931   1640416.667    121084.931 HI 3 GP1 got: -7616728.271	57271318.953   1640416.667    121084.931 HI 3 GP1  | - |
| `test_proj_nad83.yaml#73` | proj | text mismatch expected: 1631925.017	166493.704   1631925.017    166493.704 HI 3 GP2 got: -7600641.985	57224717.890   1631925.017    166493.704 HI 3 GP2  | - |
| `test_proj_nad83.yaml#74` | proj | text mismatch expected: 1640416.667	90820.525   1640416.667     90820.525 HI 4 GP1 got: -7049442.878	57125808.026   1640416.667     90820.525 HI 4 GP1  | - |
| `test_proj_nad83.yaml#75` | proj | text mismatch expected: 1640416.667	12109.121   1640416.667     12109.121 HI 5 GP1 got: -6832974.637	57303874.869   1640416.667     12109.121 HI 5 GP1  | - |
| `test_proj_nad83.yaml#76` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#77` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#78` | proj | text mismatch expected: 777180.670	778579.414    777180.670    778579.414 ID E GP1 got: -17007875.073	40520683.540    777180.670    778579.414 ID E GP1  | - |
| `test_proj_nad83.yaml#79` | proj | text mismatch expected: 600566.613	701226.817    600566.613    701226.817 ID E GP2 got: -16977423.821	40778495.094    600566.613    701226.817 ID E GP2  | - |
| `test_proj_nad83.yaml#80` | proj | text mismatch expected: 2473902.726	2357266.576   2473902.726   2357266.577 ID W GP1 got: -14463626.753	40751534.959   2473902.726   2357266.577 ID W GP1  | - |
| `test_proj_nad83.yaml#81` | proj | text mismatch expected: 1042839.901	1858837.259   1042839.901   1858837.259 IL E GP1 got: -20154472.730	32046567.000   1042839.901   1858837.259 IL E GP1  | - |
| `test_proj_nad83.yaml#82` | proj | text mismatch expected: 884532.422	1478959.911    884532.422   1478959.912 IL E GP2 got: -20738679.539	32261213.351    884532.422   1478959.912 IL E GP2  | - |
| `test_proj_nad83.yaml#83` | proj | text mismatch expected: 328083.333	1730697.447    328083.333   1730697.447 IN E GP1 got: -21046463.533	30938056.436    328083.333   1730697.447 IN E GP1  | - |
| `test_proj_nad83.yaml#84` | proj | text mismatch expected: 328083.333	1733733.065    328083.333   1733733.066 IN E GP2 got: -21041804.663	30938602.611    328083.333   1733733.066 IN E GP2  | - |
| `test_proj_nad83.yaml#85` | proj | text mismatch expected: 167175.533	1732499.995    167175.533   1732499.995 IN E GP3 got: -21071554.775	31185576.296    167175.533   1732499.995 IN E GP3  | - |
| `test_proj_nad83.yaml#86` | proj | text mismatch expected: 167187.126	1734419.540    167187.126   1734419.540 IN E GP4 got: -21068601.229	31185877.704    167187.126   1734419.540 IN E GP4  | - |
| `test_proj_nad83.yaml#87` | proj | text mismatch expected: 167173.047	1736013.615    167173.047   1736013.616 IN E GP6 got: -21066152.654	31186164.408    167173.047   1736013.616 IN E GP6  | - |
| `test_proj_nad83.yaml#88` | proj | text mismatch expected: 2952750.000	1730697.447   2952750.000   1730697.447 IN W GP1 got: -21106352.459	31549126.649   2952750.000   1730697.447 IN W GP1  | - |
| `test_proj_nad83.yaml#89` | proj | text mismatch expected: 2952750.000	1733733.065   2952750.000   1733733.066 IN W GP2 got: -21101666.163	31549496.031   2952750.000   1733733.066 IN W GP2  | - |
| `test_proj_nad83.yaml#90` | proj | text mismatch expected: 3045717.498	1618031.699   3045717.498   1618031.699 IN W GP3 got: -21269010.398	31390949.660   3045717.498   1618031.699 IN W GP3  | - |
| `test_proj_nad83.yaml#91` | proj | text mismatch expected: 2776105.988	968944.255   2776105.988    968944.255 IN W GP4 got: -22330304.897	31737487.234   2776105.988    968944.255 IN W GP4  | - |
| `test_proj_nad83.yaml#92` | proj | text mismatch expected: 3106817.690	1565874.112   3106817.690   1565874.113 IN W GP5 got: -21341856.259	31288980.923   3106817.690   1565874.113 IN W GP5  | - |
| `test_proj_nad83.yaml#93` | proj | text mismatch expected: 2855150.544	1162044.125   2855150.544   1162044.125 IN W GP6 got: -22010825.578	31633376.640   2855150.544   1162044.125 IN W GP6  | - |
| `test_proj_nad83.yaml#94` | proj | text mismatch expected: 3188649.790	1736609.724   3188649.790   1736609.724 IN W GP7 got: -21066152.654	31186164.408   3188649.790   1736609.724 IN W GP7  | - |
| `test_proj_nad83.yaml#95` | proj | text mismatch expected: 3188708.130	1733096.467   3188708.130   1733096.467 IN W GP8 got: -21071554.775	31185576.296   3188708.130   1733096.467 IN W GP8  | - |
| `test_proj_nad83.yaml#96` | proj | text mismatch expected: 3188689.210	1735016.020   3188689.210   1735016.020 IN W GP9 got: -21068601.229	31185877.704   3188689.210   1735016.020 IN W GP9  | - |
| `test_proj_nad83.yaml#97` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#98` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#99` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#100` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#101` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#102` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#103` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#104` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#105` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#106` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#107` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#108` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#109` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#110` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#111` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#112` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#113` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#114` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#115` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#116` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#117` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#118` | proj | text mismatch expected: 1007629.154	1049880.999   1007629.154   1049880.999 ME E GP1 got: -15871285.644	25788750.123   1007629.154   1049880.999 ME E GP1  | - |
| `test_proj_nad83.yaml#119` | proj | text mismatch expected: 953127.598	1228762.971    953127.598   1228762.971 ME E GP2 got: -15700645.338	25961035.181    953127.598   1228762.971 ME E GP2  | - |
| `test_proj_nad83.yaml#120` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#121` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#122` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#123` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#124` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#125` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#126` | proj | text mismatch expected: 1640416.667	879854.176   1640416.667    879854.176 MO C GP1 got: -22122994.131	33969129.944   1640416.667    879854.176 MO C GP1  | - |
| `test_proj_nad83.yaml#127` | proj | text mismatch expected: 1640416.667	882888.780   1640416.667    882888.780 MO C GP2 got: -22118127.900	33968789.029   1640416.667    882888.780 MO C GP2  | - |
| `test_proj_nad83.yaml#128` | proj | text mismatch expected: 471136.507	561031.592    471136.507    561031.592 MO E GP1 got: -22665644.934	33625911.544    471136.507    561031.592 MO E GP1  | - |
| `test_proj_nad83.yaml#129` | proj | text mismatch expected: 926703.606	386902.829    926703.606    386902.829 MO E GP2 got: -22969298.533	32880682.373    926703.606    386902.829 MO E GP2  | - |
| `test_proj_nad83.yaml#130` | proj | text mismatch expected: 2788708.333	758522.219   2788708.333    758522.219 MO W GP1 got: -22032976.012	34886548.478   2788708.333    758522.219 MO W GP1  | - |
| `test_proj_nad83.yaml#131` | proj | text mismatch expected: 2788708.333	761556.846   2788708.333    761556.846 MO W GP2 got: -22028152.211	34885939.273   2788708.333    761556.846 MO W GP2  | - |
| `test_proj_nad83.yaml#132` | proj | text mismatch expected: 878059.046	369015.468    878059.046    369015.468 MS E GP1 got: -27136711.873	32304098.924    878059.046    369015.468 MS E GP1  | - |
| `test_proj_nad83.yaml#133` | proj | text mismatch expected: 1109567.483	442842.466   1109567.483    442842.466 MS E GP2 got: -26974849.332	31859086.997   1109567.483    442842.466 MS E GP2  | - |
| `test_proj_nad83.yaml#134` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#135` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#136` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#137` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#138` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#139` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#140` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#141` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#142` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#143` | proj | text mismatch expected: 1179151.981	233188.619   1179151.981    233188.620 NH   GP1 got: -17752211.045	25795127.749   1179151.981    233188.620 NH   GP1  | - |
| `test_proj_nad83.yaml#144` | proj | text mismatch expected: 749470.166	131406.173    749470.166    131406.173 NH   GP2 got: -18139020.339	26270833.693    749470.166    131406.173 NH   GP2  | - |
| `test_proj_nad83.yaml#145` | proj | text mismatch expected: 567304.543	376673.733    567304.543    376673.733 NJ   GP1 got: -19841378.603	26237020.105    567304.543    376673.733 NJ   GP1  | - |
| `test_proj_nad83.yaml#146` | proj | text mismatch expected: 393979.614	863010.549    393979.614    863010.549 NJ   GP2 got: -19283957.175	26749690.379    393979.614    863010.549 NJ   GP2  | - |
| `test_proj_nad83.yaml#147` | proj | text mismatch expected: 1640416.667	909448.493   1640416.667    909448.493 NM C GP1 got: -23024480.407	41163615.152   1640416.667    909448.493 NM C GP1  | - |
| `test_proj_nad83.yaml#148` | proj | text mismatch expected: 1640416.667	912480.595   1640416.667    912480.595 NM C GP2 got: -23020008.458	41161273.237   1640416.667    912480.595 NM C GP2  | - |
| `test_proj_nad83.yaml#149` | proj | text mismatch expected: 583573.491	832847.194    583573.491    832847.194 NM E GP1 got: -23604810.992	40272302.962    583573.491    832847.194 NM E GP1  | - |
| `test_proj_nad83.yaml#150` | proj | text mismatch expected: 400747.149	864523.566    400747.149    864523.566 NM E GP2 got: -23425459.054	40530920.080    400747.149    864523.566 NM E GP2  | - |
| `test_proj_nad83.yaml#151` | proj | text mismatch expected: 2723091.667	545634.896   2723091.667    545634.896 NM W GP1 got: -23159887.724	42167339.939   2723091.667    545634.896 NM W GP1  | - |
| `test_proj_nad83.yaml#152` | proj | text mismatch expected: 2723091.667	548666.562   2723091.667    548666.562 NM W GP2 got: -23155523.738	42164749.674   2723091.667    548666.562 NM W GP2  | - |
| `test_proj_nad83.yaml#153` | proj | text mismatch expected: 882966.545	26600313.129    882966.545  26600313.129 NV E GP1 got: -19762724.542	43862359.904    882966.545  26600313.129 NV E GP1  | - |
| `test_proj_nad83.yaml#154` | proj | text mismatch expected: 311338.993	28710910.564    311338.993  28710910.565 NV E GP2 got: -16916328.237	42689781.243    311338.993  28710910.565 NV E GP2  | - |
| `test_proj_nad83.yaml#155` | proj | text mismatch expected: 614362.369	1257287.611    614362.369   1257287.611 NY E GP1 got: -18639375.586	26674646.007    614362.369   1257287.611 NY E GP1  | - |
| `test_proj_nad83.yaml#156` | proj | text mismatch expected: 426225.275	1336579.561    426225.275   1336579.561 NY E GP2 got: -18636854.163	26964743.734    426225.275   1336579.561 NY E GP2  | - |
| `test_proj_nad83.yaml#157` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#158` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#159` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#160` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#161` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#162` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#163` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#164` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#165` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#166` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#167` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#168` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#169` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#170` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#171` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#172` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#173` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#174` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#175` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#176` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#177` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#178` | proj | text mismatch expected: 391898.667	166566.121    391898.667    166566.121 RI   GP1 got: -18534408.230	25538871.442    391898.667    166566.121 RI   GP1  | - |
| `test_proj_nad83.yaml#179` | proj | text mismatch expected: 295027.785	114722.837    295027.785    114722.837 RI   GP2 got: -18661994.096	25628506.875    295027.785    114722.837 RI   GP2  | - |
| `test_proj_nad83.yaml#180` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#181` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#182` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#183` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#184` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#185` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#186` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#187` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#188` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#189` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#190` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#191` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#192` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#193` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#194` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#195` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#196` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#197` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#198` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#199` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#200` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#201` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#202` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#203` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#204` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#205` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#206` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#207` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#208` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#209` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#210` | proj | text mismatch expected: 1642534.834	242819.594   1642534.834    242819.594 VT   GP1 got: -17988569.951	26319458.284   1642534.834    242819.594 VT   GP1  | - |
| `test_proj_nad83.yaml#211` | proj | text mismatch expected: 1456873.971	683480.189   1456873.971    683480.189 VT   GP2 got: -17539083.121	26802663.245   1456873.971    683480.189 VT   GP2  | - |
| `test_proj_nad83.yaml#212` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#213` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#214` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#215` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#216` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#217` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#218` | proj | exit 3 expected 0 proj: illegal value for +lat_1  | - |
| `test_proj_nad83.yaml#219` | proj | text mismatch expected: 563107.342	1097477.489    563107.342   1097477.489 WY E GP1 got: -18119379.866	38563102.968    563107.342   1097477.489 WY E GP1  | - |
| `test_proj_nad83.yaml#220` | proj | text mismatch expected: 599946.619	1094729.118    599946.619   1094729.119 WY E GP2 got: -18141901.794	38516673.836    599946.619   1094729.119 WY E GP2  | - |
| `test_proj_nad83.yaml#221` | proj | text mismatch expected: 573561.723	1133155.513    573561.723   1133155.514 WY E GP3 got: -18078474.219	38531194.482    573561.723   1133155.514 WY E GP3  | - |
| `test_proj_nad83.yaml#222` | proj | text mismatch expected: 594028.794	552611.396    594028.794    552611.396 WY E GP4 got: -18850340.698	38809706.057    594028.794    552611.396 WY E GP4  | - |
| `test_proj_nad83.yaml#223` | proj | text mismatch expected: 812768.620	758647.940    812768.620    758647.940 WY E GP5 got: -18692835.854	38410770.499    812768.620    758647.940 WY E GP5  | - |
| `test_proj_nad83.yaml#224` | proj | text mismatch expected: 1617477.770	732300.770   1617477.770    732300.770 WYEC GP1 got: -18923551.397	39189426.141   1617477.770    732300.770 WYEC GP1  | - |
| `test_proj_nad83.yaml#225` | proj | text mismatch expected: 1121920.469	826536.345   1121920.469    826536.345 WYEC GP2 got: -18504663.353	39773388.788   1121920.469    826536.345 WYEC GP2  | - |
