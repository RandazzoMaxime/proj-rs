# Unported CLI cases

The yaml runner executes `cct`, `cs2cs`, `projinfo`, `proj`, and `invproj`.
A row remains when that case does not match the yaml.

| id | app | reason | comment |
|---|---|---|---|
| `test_cs2cs_datumfile.yaml#3` | cs2cs | no numbers expected 40d30'0.368"N	59d59'56.617"W 0.000 got *	* inf | Test --bbox -141.01,40.04,-47.74,83.17 NAD27 NAD83 (using NTv2) |
| `test_cs2cs_ignf.yaml#9` | cs2cs | no numbers expected 1238837.253	5057451.037 0.000 got *	* inf | - |
| `test_cs2cs_ignf.yaml#17` | cs2cs | no numbers expected 659421.855	3603178.891 0.000 got *	* inf | - |
