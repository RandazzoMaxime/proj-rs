"""Copy the unit tests and doc examples of georust/proj into a crate that depends on proj-rs."""
import pathlib, re, sys

src, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])

def test_module(text):
    match = re.search(r"#\[cfg\(test\)\]\s*mod tests?\s*\{", text)
    if not match:
        return None
    body = text[match.end():].rstrip()
    assert body.endswith("}")
    body = body[:-1]
    imports = {
        "proj": "use proj::*;\nuse proj::wkt::{WktOptions, WktOutputAxis, WktVersion};",
        "geo_types": "use proj::*;\nuse geo_types::{coord, Geometry};",
    }
    body = body.replace("use super::*;", imports[NAME])
    # A test that builds WktOptions from its private fields only compiles inside georust.
    internal = '#[ignore = "builds WktOptions from private fields: only possible inside georust"]\n    '
    body = re.sub(r"#\[test\]\n(\s*)(fn \w+\(\)\s*\{(?:(?!\n    #\[).)*?WktOptions \{)",
                  lambda m: "#[test]\n" + m.group(1) + internal + m.group(2), body, flags=re.S)
    body = re.sub(r"WktOptions \{[^}]*\.\.Default::default\(\)\s*\}", "WktOptions::default()", body)
    return body

for name in ["proj", "geo_types"]:
    NAME = name
    body = test_module((src / "src" / f"{name}.rs").read_text())
    (out / "tests" / f"georust_{name}.rs").write_text(
        "#![allow(unused, clippy::all)]\n#[macro_use]\nextern crate approx;\n" + body + "\n")

items = []
for path in sorted((src / "src").glob("*.rs")):
    block = []
    for line in path.read_text().splitlines() + [""]:
        stripped = line.strip()
        if stripped.startswith("///") or stripped.startswith("//!"):
            block.append("///" + stripped[3:])
            continue
        if any("```" in doc for doc in block):
            items.append((path.name, block))
        block = []
    # Examples behind `cfg_attr(feature = "geo-types", doc = r##"..."##)`, which the feature turns on.
    for raw in re.findall(r'doc\s*=\s*r##"(.*?)"##', path.read_text(), flags=re.S):
        if "```" in raw:
            items.append((path.name, ["///" + line for line in raw.splitlines()]))
lib = ["#![allow(rustdoc::all)]"]
for index, (name, block) in enumerate(items):
    lib.append(f"/// From `src/{name}` of georust/proj.")
    lib.extend(block)
    lib.append(f"pub fn doc_{index}() {{}}")
(out / "src" / "lib.rs").write_text("\n".join(lib) + "\n")
print(f"{len(items)} documented items with examples")
