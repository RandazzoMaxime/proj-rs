//! What PROJ reports about the library and its files: `proj_info` (version and
//! search path), `proj_grid_info` (a grid file) and `proj_init_info` (the metadata
//! of an init file). The functions are those of PROJ 9.9.0 (`src/info.cpp` and
//! `src/grids.cpp`).
//!
//! PROJ returns a struct of empty strings for a file it does not find. Here that
//! is `None`.

use std::path::PathBuf;

use crate::grids::{Grid, HorizontalShiftGridSet, VerticalShiftGridSet};
use crate::proj::{port_info, Info};

/// `proj_info()`: the version of the library and its default search path.
///
/// The search path is the directories added to the default context, or else the
/// data directories, which are `PROJ_DATA` (or the legacy `PROJ_LIB`) and the
/// default data directory. Each one is separated by `:`, or `;` on Windows. PROJ's
/// user-writable directory is not in the list.
#[must_use]
pub fn library_info() -> Info {
    port_info(&crate::network::default_searchpath())
}

/// A point of a grid, in radians for a geographic grid. `PJ_LP` in PROJ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    /// Longitude.
    pub lon: f64,
    /// Latitude.
    pub lat: f64,
}

/// `PJ_GRID_INFO`: a grid file, described by its first grid.
#[derive(Clone, Debug, PartialEq)]
pub struct GridFileInfo {
    /// The name the grid was asked by.
    pub gridname: String,
    /// The file the name resolves to. `None` when it is not a local file.
    pub filename: Option<PathBuf>,
    /// The file format: `"gtiff"`, `"gtx"`, `"ntv1"`, `"ntv2"` or `"ctable2"`.
    pub format: String,
    /// Columns of the grid.
    pub n_lon: usize,
    /// Rows of the grid.
    pub n_lat: usize,
    /// Cell size along the longitudes.
    pub cs_lon: f64,
    /// Cell size along the latitudes.
    pub cs_lat: f64,
    /// South-west corner of the bounds.
    pub lower_left: Corner,
    /// North-east corner of the bounds.
    pub upper_right: Corner,
}

/// `proj_grid_info(gridname)`: the grid file a name gives.
///
/// Vertical grids are tried first, then horizontal ones, as PROJ does. `None`
/// when the name is not on the search path, or when the file is not a grid the
/// crate reads.
#[must_use]
pub fn grid_info(name: &str) -> Option<GridFileInfo> {
    if let Ok(set) = VerticalShiftGridSet::open(name) {
        if let Some(info) = describe_grid(name, set.format(), &set.grids()) {
            return Some(info);
        }
    }
    let set = HorizontalShiftGridSet::open(name).ok()?;
    describe_grid(name, set.format(), &set.grids())
}

fn describe_grid(name: &str, format: &str, grids: &[Grid<'_>]) -> Option<GridFileInfo> {
    let grid = grids.first()?;
    let extent = grid.extent();
    Some(GridFileInfo {
        gridname: name.to_owned(),
        filename: crate::gridpath::resolve(name),
        format: format.to_owned(),
        n_lon: grid.width(),
        n_lat: grid.height(),
        cs_lon: extent.res_x,
        cs_lat: extent.res_y,
        lower_left: Corner {
            lon: extent.west,
            lat: extent.south,
        },
        upper_right: Corner {
            lon: extent.east,
            lat: extent.north,
        },
    })
}

/// `PJ_INIT_INFO`: the metadata of an init file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitInfo {
    /// The init name, as asked.
    pub name: String,
    /// The file the name resolves to. `None` for `epsg` and `IGNF`, whose
    /// metadata comes from the database.
    pub filename: Option<PathBuf>,
    /// Where the definitions come from. A file's `metadata` section gives it;
    /// `Unknown` when the section is missing.
    pub origin: String,
    /// Version of the file, or of the database table, empty when unknown.
    pub version: String,
    /// Date of the last update, empty when unknown.
    pub lastupdate: String,
}

/// `proj_init_info(name)`: the metadata of an init file.
///
/// The file is searched on the path like any other name. `epsg` and `IGNF` may
/// have no file: their metadata then comes from the database. `None` when the
/// name is neither a file nor one of those two.
#[must_use]
pub fn init_info(name: &str) -> Option<InitInfo> {
    if name.len() <= 64 {
        if let Some(path) = crate::network::find_file(name) {
            return Some(file_init_info(name, path));
        }
    }
    match name {
        "epsg" | "EPSG" => Some(database_init_info(name, "EPSG")),
        "IGNF" => Some(database_init_info(name, "IGNF")),
        _ => None,
    }
}

/// The metadata of a file: its `metadata` section, read as PROJ reads an init
/// section. A missing section leaves `Unknown` in place.
fn file_init_info(name: &str, path: PathBuf) -> InitInfo {
    let mut info = InitInfo {
        name: name.to_owned(),
        filename: Some(path.clone()),
        origin: "Unknown".to_owned(),
        version: "Unknown".to_owned(),
        lastupdate: "Unknown".to_owned(),
    };
    let section = crate::network::read_file(&path)
        .ok()
        .and_then(|bytes| section_text(&bytes, "metadata"));
    if let Some(section) = section {
        if let Some(origin) = parameter(&section, "origin") {
            info.origin = origin.to_owned();
        }
        if let Some(version) = parameter(&section, "version") {
            info.version = version.to_owned();
        }
        if let Some(lastupdate) = parameter(&section, "lastupdate") {
            info.lastupdate = lastupdate.to_owned();
        }
    }
    info
}

/// The metadata of `epsg` or `IGNF` from the database: its version and date rows.
fn database_init_info(name: &str, origin: &str) -> InitInfo {
    let database = crate::DatabaseContext::create().ok();
    let lookup = |key: &str| {
        database
            .as_ref()
            .and_then(|db| db.metadata(&format!("{origin}.{key}")).ok().flatten())
            .unwrap_or_default()
    };
    InitInfo {
        name: name.to_owned(),
        filename: None,
        origin: origin.to_owned(),
        version: lookup("VERSION"),
        lastupdate: lookup("DATE"),
    }
}

/// `pj_chomp`: cuts a `#` comment, then strips the blanks and `;` at both ends.
fn chomp(line: &str) -> &str {
    let uncommented = line.split('#').next().unwrap_or("");
    uncommented.trim_matches(|c: char| c.is_whitespace() || c == ';')
}

/// Whether a line opens the section `section`: `<section>` at its start.
fn opens_section(line: &str, section: &str) -> bool {
    let bytes = line.as_bytes();
    let n = section.len();
    bytes.first() == Some(&b'<')
        && bytes.get(n + 1) == Some(&b'>')
        && bytes.get(1..=n) == Some(section.as_bytes())
}

/// The text of the `<section>` of an init file, as PROJ's `get_init_string` reads
/// it. It starts after the header and runs to the next `<`. The lines of the
/// section are joined by blanks, after `chomp`.
fn section_text(bytes: &[u8], section: &str) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines().map(chomp);
    let header = lines.find(|line| opens_section(line, section))?;
    let mut buffer = header[section.len() + 2..].to_owned();
    loop {
        if let Some(cut) = buffer.find('<') {
            buffer.truncate(cut);
            break;
        }
        match lines.next() {
            Some(line) => {
                buffer.push(' ');
                buffer.push_str(line);
            }
            None => break,
        }
    }
    Some(buffer)
}

/// The value of `key` in the text of a section: `key=value` items, with or
/// without a leading `+`. The first item with the key wins, as `pj_param` does.
fn parameter<'a>(section: &'a str, key: &str) -> Option<&'a str> {
    section.split_whitespace().find_map(|item| {
        let item = item.strip_prefix('+').unwrap_or(item);
        let (name, value) = item.split_once('=')?;
        (name == key).then_some(value)
    })
}

#[cfg(test)]
mod tests {
    use super::{chomp, parameter, section_text};

    #[test]
    fn chomp_drops_comments_and_outer_blanks() {
        assert_eq!(chomp("  +proj=utm ; # note"), "+proj=utm");
        assert_eq!(chomp("# only a comment"), "");
    }

    #[test]
    fn section_runs_to_the_next_angle_bracket() {
        let text = b"# head\n<metadata> version=1.2 # ignored\norigin=Test\n<next> +proj=utm\n";
        assert_eq!(
            section_text(text, "metadata").as_deref(),
            Some(" version=1.2 origin=Test ")
        );
        assert_eq!(section_text(b"<other> x=1\n", "metadata"), None);
    }

    #[test]
    fn parameter_takes_the_first_item_with_the_key() {
        let section = " +version=1.2 version=9 origin=Test";
        assert_eq!(parameter(section, "version"), Some("1.2"));
        assert_eq!(parameter(section, "origin"), Some("Test"));
        assert_eq!(parameter(section, "lastupdate"), None);
    }
}
