//! The PROJ pipeline text an operation exports: the steps `PROJStringFormatter`
//! collects, the inversion stack, and the rules its `toString()` applies
//! before printing.
//!
//! Not ported from `io.cpp`: the grid push/pop rewriting (`hgridshift`,
//! `vgridshift`, and the `omit_fwd`/`omit_inv` flags it sets), the Krovak
//! `axis=swu` merge, and the `set` epoch merge. Skipping one keeps the same
//! transform with more steps, so the text differs from PROJ on those inputs.
//! The PROJ string parser reads `init=` as a parameter and drops global parameters.
//! Values are written without quoting, which is enough for the numbers and
//! names an operation produces.

use super::wkt::format_g;

#[cfg(test)]
mod tests;

/// Line breaks in a PROJ string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjLines {
    /// One step per line, indented under `+proj=pipeline`.
    Multi,
    /// A single line.
    Single,
}

/// The layout of a PROJ pipeline text: the options of `proj_as_proj_string`.
///
/// PROJ reads `MULTILINE` (default `NO`), `INDENTATION_WIDTH` (default 2) and `MAX_LINE_LENGTH`
/// (default 80). A line that would pass `max_line_length` columns is broken before its next
/// parameter, which then starts under the step's own `+step ` text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjStringLayout {
    /// One line, `MULTILINE=NO`.
    Single,
    /// One step per line, `MULTILINE=YES`.
    Multi {
        /// Spaces before each `+step` line, `INDENTATION_WIDTH`.
        indentation_width: usize,
        /// Longest line before a parameter moves to the next line, `MAX_LINE_LENGTH`.
        max_line_length: usize,
    },
}

impl From<ProjLines> for ProjStringLayout {
    fn from(lines: ProjLines) -> Self {
        match lines {
            ProjLines::Single => Self::Single,
            ProjLines::Multi => Self::Multi {
                indentation_width: 2,
                max_line_length: 80,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Param {
    key: String,
    /// Empty for a flag such as `+exact`.
    value: String,
}

impl Param {
    fn is(&self, key: &str, value: &str) -> bool {
        self.key == key && self.value == value
    }

    fn keyed(&self, key: &str) -> bool {
        self.key == key
    }
}

#[derive(Clone, Debug)]
struct Step {
    name: String,
    inverted: bool,
    params: Vec<Param>,
}

/// Collects the steps of one operation, then prints them.
#[derive(Debug, Default)]
pub(crate) struct PipelineFormatter {
    steps: Vec<Step>,
    /// Index of the first step of each open inversion.
    inversions: Vec<usize>,
    /// Drops `unitconvert` steps that change nothing, as PROJ does once a
    /// Helmert step is written.
    optimize_units: bool,
    /// `(official name, PROJ name, inverse direction)` of each grid the database renames.
    grid_names: Vec<(String, String, bool)>,
    /// Whether the grid and deformation methods write their steps. Off, they are
    /// unimplemented, so the candidate selection of `cs2cs` is unchanged.
    grid_steps: bool,
    /// Open `pushOmitZUnitConversion` scopes.
    omit_z_unit_depth: usize,
    /// Open `pushOmitHorizontalConversionInVertTransformation` scopes.
    omit_horizontal_depth: usize,
}

impl PipelineFormatter {
    /// A formatter that writes each grid under its PROJ name, as
    /// `substitutePROJAlternativeGridNames` does with a database context.
    pub(crate) fn with_grid_names(grid_names: Vec<(String, String, bool)>) -> Self {
        Self {
            grid_names,
            ..Self::default()
        }
    }

    /// Lets the grid and deformation methods write their steps.
    pub(crate) fn with_grid_steps(mut self) -> Self {
        self.grid_steps = true;
        self
    }

    pub(crate) fn grid_steps(&self) -> bool {
        self.grid_steps
    }

    pub(crate) fn push_omit_z_unit_conversion(&mut self) {
        self.omit_z_unit_depth += 1;
    }

    pub(crate) fn pop_omit_z_unit_conversion(&mut self) {
        self.omit_z_unit_depth = self.omit_z_unit_depth.saturating_sub(1);
    }

    pub(crate) fn omits_z_unit_conversion(&self) -> bool {
        self.omit_z_unit_depth > 0
    }

    pub(crate) fn push_omit_horizontal_in_vertical(&mut self) {
        self.omit_horizontal_depth += 1;
    }

    pub(crate) fn pop_omit_horizontal_in_vertical(&mut self) {
        self.omit_horizontal_depth = self.omit_horizontal_depth.saturating_sub(1);
    }

    pub(crate) fn omits_horizontal_in_vertical(&self) -> bool {
        self.omit_horizontal_depth > 0
    }

    /// The name a grid is written under.
    pub(crate) fn grid_name(&self, name: &str) -> String {
        self.grid_names
            .iter()
            .find(|(original, _, _)| original == name)
            .map_or_else(|| name.to_owned(), |(_, proj, _)| proj.clone())
    }

    /// Whether the grid is written as the inverse of its PROJ grid, `inverse_direction` in
    /// `grid_alternatives`.
    pub(crate) fn grid_is_inverse(&self, name: &str) -> bool {
        self.grid_names
            .iter()
            .any(|(original, _, inverse)| original == name && *inverse)
    }

    pub(crate) fn add_step(&mut self, name: &str) {
        self.steps.push(Step {
            name: name.to_owned(),
            inverted: false,
            params: Vec::new(),
        });
    }

    /// Marks the last step as inverted.
    pub(crate) fn invert_last_step(&mut self) {
        if let Some(step) = self.steps.last_mut() {
            step.inverted = true;
        }
    }

    /// PROJ `PROJStringFormatter::ingestPROJString`: appends the steps of a PROJ string.
    ///
    /// A pipeline string, or one step string such as `+proj=axisswap +order=2,1`.
    pub(crate) fn ingest_proj_string(&mut self, text: &str) {
        self.steps.extend(parse_proj_string(text));
    }

    pub(crate) fn add_param(&mut self, key: &str, value: &str) {
        self.push_param(key, value.to_owned());
    }

    pub(crate) fn add_flag(&mut self, key: &str) {
        self.push_param(key, String::new());
    }

    pub(crate) fn add_number(&mut self, key: &str, value: f64) {
        self.push_param(key, proj_number(value));
    }

    fn push_param(&mut self, key: &str, value: String) {
        if self.steps.is_empty() {
            self.add_step("");
        }
        if let Some(step) = self.steps.last_mut() {
            step.params.push(Param {
                key: key.to_owned(),
                value,
            });
        }
    }

    /// Everything added until [`Self::stop_inversion`] runs backwards.
    pub(crate) fn start_inversion(&mut self) {
        self.inversions.push(self.steps.len());
    }

    pub(crate) fn stop_inversion(&mut self) {
        let Some(start) = self.inversions.pop() else {
            return;
        };
        for step in &mut self.steps[start..] {
            step.inverted = !step.inverted;
            for param in &mut step.params {
                if param.is("omit_fwd", "") {
                    "omit_inv".clone_into(&mut param.key);
                } else if param.is("omit_inv", "") {
                    "omit_fwd".clone_into(&mut param.key);
                }
            }
        }
        self.steps[start..].reverse();
    }

    pub(crate) fn set_optimize_units(&mut self) {
        self.optimize_units = true;
    }

    /// Runs the optimizations and prints the pipeline.
    pub(crate) fn finish(self, lines: ProjLines) -> String {
        self.finish_layout(lines.into())
    }

    /// [`Self::finish`] with the full layout of `proj_as_proj_string`.
    pub(crate) fn finish_layout(mut self, layout: ProjStringLayout) -> String {
        self.drop_noops();
        self.normalize_inverses();
        self.merge_adjacent();
        self.rewrite_patterns();
        self.render(layout)
    }

    /// The pattern rewrites of PROJ's `PROJStringFormatter`, run until none applies: grid
    /// push/pop, the axis and unit moves around `push`, and the Krovak `axis=swu` merge.
    fn rewrite_patterns(&mut self) {
        while rewrite_once(&mut self.steps) {}
    }

    fn drop_noops(&mut self) {
        let mut index = 0;
        while index < self.steps.len() {
            let len = self.steps.len();
            let step = &self.steps[index];
            let remove = (step.name == "helmert" && zero_helmert(&step.params))
                || (self.optimize_units && identity_unitconvert(step))
                || (step.name == "noop" && len > 1);
            if remove {
                self.steps.remove(index);
                continue;
            }
            let step = &mut self.steps[index];
            if step.inverted && step.name == "push" {
                step.name = "pop".to_owned();
                step.inverted = false;
            } else if step.inverted && step.name == "pop" {
                step.name = "push".to_owned();
                step.inverted = false;
            }
            index += 1;
        }
    }

    fn normalize_inverses(&mut self) {
        for step in self.steps.iter_mut().filter(|step| step.inverted) {
            normalize_inverse(step);
        }
    }

    fn merge_adjacent(&mut self) {
        let mut cur = 1;
        while cur < self.steps.len() {
            cur = match adjacent_rule(&mut self.steps, cur) {
                Some(next) => next,
                None => cur + 1,
            };
        }
    }

    fn render(self, layout: ProjStringLayout) -> String {
        let (multi, indentation, max_line_length) = match layout {
            ProjStringLayout::Single => (false, 0, usize::MAX),
            ProjStringLayout::Multi {
                indentation_width,
                max_line_length,
            } => (true, indentation_width, max_line_length),
        };
        let mut result = String::new();
        // Columns before each `+step` line: the `INDENTATION_WIDTH` under `+proj=pipeline`.
        let mut indent = 0;
        if self.steps.len() > 1 || self.steps.first().is_some_and(|step| step.inverted) {
            result.push_str("+proj=pipeline");
            if multi {
                indent = indentation;
            }
        }
        for step in &self.steps {
            let mut line = if result.is_empty() {
                String::new()
            } else if multi {
                format!("{}+step", " ".repeat(indent))
            } else {
                " +step".to_owned()
            };
            if step.inverted {
                line.push_str(" +inv");
            }
            if !step.name.is_empty() {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str("+proj=");
                line.push_str(&step.name);
            }
            for param in &step.params {
                let mut kv = format!("+{}", param.key);
                if !param.value.is_empty() {
                    kv.push('=');
                    kv.push_str(&param.value);
                }
                if multi && line.len() + kv.len() > max_line_length {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&line);
                    line = " ".repeat(indent + "+step ".len());
                } else if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(&kv);
            }
            if multi && !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&line);
        }
        if result.is_empty() {
            "+proj=noop".to_owned()
        } else {
            result
        }
    }
}

/// A PROJ pipeline kept as its steps, so that it can be inverted and written with any layout.
///
/// `proj_coordoperation_create_inverse` inverts the operation of a pipeline and writes it back with
/// `proj_as_proj_string`. Global parameters and `title=` are not kept, as in
/// [`parse_proj_string`].
#[derive(Clone, Debug, Default)]
pub(crate) struct Pipeline {
    steps: Vec<Step>,
}

impl Pipeline {
    /// The steps of a PROJ pipeline text, or of one step.
    pub(crate) fn parse(text: &str) -> Self {
        Self {
            steps: parse_proj_string(text),
        }
    }

    /// The inverse of the steps: the same steps in reverse order, each one inverted.
    ///
    /// A `push` (or `pop`) that becomes inverted is written as its opposite without `+inv`, as
    /// PROJ writes `pop` for `inv +proj=push`. The other inversions are left to the formatter,
    /// which drops those it can print without `+inv` (`normalize_inverses`).
    pub(crate) fn inverse(&self) -> Self {
        let steps = self
            .steps
            .iter()
            .rev()
            .map(|step| {
                let mut inverse = step.clone();
                inverse.inverted = !step.inverted;
                if inverse.inverted && matches!(step.name.as_str(), "push" | "pop") {
                    inverse.name = if step.name == "push" { "pop" } else { "push" }.to_owned();
                    inverse.inverted = false;
                }
                inverse
            })
            .collect();
        Self { steps }
    }

    /// The text of the steps, as `PROJStringFormatter` writes them with `layout`.
    pub(crate) fn render(&self, layout: ProjStringLayout) -> String {
        PipelineFormatter {
            steps: self.steps.clone(),
            ..PipelineFormatter::default()
        }
        .finish_layout(layout)
    }
}

fn zero_helmert(params: &[Param]) -> bool {
    let count = params.len();
    if count != 3 && count != 8 {
        return false;
    }
    params[0].is("x", "0")
        && params[1].is("y", "0")
        && params[2].is("z", "0")
        && (count == 3
            || (params[3].is("rx", "0")
                && params[4].is("ry", "0")
                && params[5].is("rz", "0")
                && params[6].is("s", "0")
                && params[7].keyed("convention")))
}

fn identity_unitconvert(step: &Step) -> bool {
    step.name == "unitconvert"
        && step.params.len() == 2
        && step.params[0].keyed("xy_in")
        && step.params[1].keyed("xy_out")
        && step.params[0].value == step.params[1].value
}

/// Rewrites an inverted step that PROJ can print without `+inv`.
fn normalize_inverse(step: &mut Step) {
    let count = step.params.len();
    match step.name.as_str() {
        "axisswap" if count == 1 => {
            let param = &step.params[0];
            if param.is("order", "2,1") || param.is("order", "1,-2") || param.is("order", "1,2,-3")
            {
                step.inverted = false;
            } else if param.is("order", "2,-1") {
                step.inverted = false;
                step.params[0].value = "-2,1".to_owned();
            }
        }
        "unitconvert" if count == 2 => {
            let swappable = (step.params[0].keyed("xy_in") && step.params[1].keyed("xy_out"))
                || (step.params[0].keyed("z_in") && step.params[1].keyed("z_out"));
            if swappable {
                let (head, tail) = step.params.split_at_mut(1);
                std::mem::swap(&mut head[0].value, &mut tail[0].value);
                step.inverted = false;
            }
        }
        "unitconvert" if count == 4 && unitconvert_xyz_keys(&step.params) => {
            let values: Vec<String> = step.params.iter().map(|p| p.value.clone()).collect();
            step.params[0].value = values[2].clone();
            step.params[2].value = values[0].clone();
            step.params[1].value = values[3].clone();
            step.params[3].value = values[1].clone();
            step.inverted = false;
        }
        "geogoffset" => {
            for param in &mut step.params {
                if matches!(param.key.as_str(), "dlat" | "dlon" | "dh") {
                    if let Ok(value) = param.value.parse::<f64>() {
                        param.value = proj_number(0.0 - value);
                    }
                }
            }
            step.inverted = false;
        }
        "set" => step.inverted = false,
        _ => {}
    }
}

/// PROJ `PROJStringSyntaxParser`, for a string with or without `+proj=pipeline`.
///
/// Tokens are split on whitespace with the leading `+` removed. Global parameters
/// (before the first step) and `title=` are dropped. `init=` is read as a parameter, so
/// it prints differently from PROJ.
fn parse_proj_string(text: &str) -> Vec<Step> {
    let tokens: Vec<&str> = text
        .split_whitespace()
        .map(|token| token.strip_prefix('+').unwrap_or(token))
        .collect();
    let mut steps: Vec<Step> = Vec::new();
    if tokens.contains(&"proj=pipeline") {
        let mut in_pipeline = false;
        let mut invert_all = false;
        for word in tokens {
            if word == "proj=pipeline" {
                in_pipeline = true;
            } else if word == "step" {
                steps.push(new_step(""));
            } else if word == "inv" {
                match steps.last_mut() {
                    Some(step) => step.inverted = true,
                    None => invert_all = true,
                }
            } else if let Some(name) = word.strip_prefix("proj=").filter(|_| in_pipeline) {
                match steps.last_mut() {
                    Some(step) if step.name.is_empty() => step.name = name.to_owned(),
                    _ => push_param(&mut steps, word),
                }
            } else {
                push_param(&mut steps, word);
            }
        }
        if invert_all && !steps.is_empty() {
            for step in &mut steps {
                step.inverted = !step.inverted;
            }
            steps.reverse();
        }
        return steps;
    }
    let has_proj = tokens.iter().any(|word| word.starts_with("proj="));
    if has_proj {
        steps.push(new_step(""));
    }
    for word in tokens {
        if let Some(name) = word.strip_prefix("proj=") {
            match steps.last_mut() {
                Some(step) if step.name.is_empty() => step.name = name.to_owned(),
                _ => push_param(&mut steps, word),
            }
        } else if word == "inv" {
            if let Some(step) = steps.last_mut() {
                step.inverted = true;
            }
        } else if word == "step" || word.starts_with("title=") {
            continue;
        } else {
            push_param(&mut steps, word);
        }
    }
    steps
}

fn new_step(name: &str) -> Step {
    Step {
        name: name.to_owned(),
        inverted: false,
        params: Vec::new(),
    }
}

/// Appends `token` (`key` or `key=value`) to the last step. A token before any step is
/// a global parameter, which is dropped.
fn push_param(steps: &mut [Step], token: &str) {
    let (key, value) = token.split_once('=').unwrap_or((token, ""));
    if let Some(step) = steps.last_mut() {
        step.params.push(Param {
            key: key.to_owned(),
            value: value.to_owned(),
        });
    }
}

fn unitconvert_xyz_keys(params: &[Param]) -> bool {
    params[0].keyed("xy_in")
        && params[1].keyed("z_in")
        && params[2].keyed("xy_out")
        && params[3].keyed("z_out")
}

/// Applies the first rule that matches the pair `(cur - 1, cur)`.
///
/// Returns the index to resume at, or `None` when no rule applies.
/// One rewrite of the PROJ formatter's pattern passes, applied at the first position that
/// matches. `true` when the steps changed.
fn rewrite_once(steps: &mut Vec<Step>) -> bool {
    (1..steps.len()).any(|cur| {
        rewrite_krovak_swu(steps, cur)
            || rewrite_grid_push_pop(steps, cur)
            || rewrite_push_axes_units(steps, cur)
    })
}

fn rewrite_krovak_swu(steps: &mut Vec<Step>, cur: usize) -> bool {
    let prev = cur - 1;
    let (p, c) = (&steps[prev], &steps[cur]);
    if p.inverted == c.inverted || p.name != c.name {
        return false;
    }
    let (sw, en) = if p.params.len() == c.params.len() + 1 {
        (p, c)
    } else if c.params.len() == p.params.len() + 1 {
        (c, p)
    } else {
        return false;
    };
    let leading_swu = sw
        .params
        .first()
        .is_some_and(|param| param.is("axis", "swu"));
    if !leading_swu || en.params.iter().zip(&sw.params[1..]).any(|(a, b)| a != b) {
        return false;
    }
    steps[cur] = Step {
        name: "axisswap".to_owned(),
        inverted: false,
        params: vec![Param {
            key: "order".to_owned(),
            value: "-2,-1".to_owned(),
        }],
    };
    steps.remove(prev);
    true
}

fn rewrite_grid_push_pop(steps: &mut Vec<Step>, cur: usize) -> bool {
    let (Some(prev), Some(next)) = (cur.checked_sub(1), steps.get(cur + 1)) else {
        return false;
    };
    let p = &steps[prev];
    if p.name != "hgridshift"
        || p.params.len() != 1
        || steps[cur].name != "vgridshift"
        || next.name != "hgridshift"
        || next.inverted == p.inverted
        || next.params.len() != 1
        || next.params[0] != p.params[0]
    {
        return false;
    }
    let next = cur + 1;
    steps[prev].params.push(flag("omit_inv"));
    steps[next].params.push(flag("omit_fwd"));
    steps.insert(next + 1, pattern_step("pop", &["v_1", "v_2"]));
    steps.insert(prev, pattern_step("push", &["v_1", "v_2"]));
    true
}

/// `axisswap` / `unitconvert` / `push` moves: a `push` is pulled in front of the axis and
/// unit steps that surround a `vgridshift`, and the repeated steps are dropped.
fn rewrite_push_axes_units(steps: &mut Vec<Step>, cur: usize) -> bool {
    let Some(prev) = cur.checked_sub(1) else {
        return false;
    };
    let at = |index: usize, name: &str| steps.get(index).is_some_and(|s| s.name == name);
    let swap21 = |index: usize| {
        steps.get(index).is_some_and(|s| {
            s.name == "axisswap" && s.params.len() == 1 && s.params[0].is("order", "2,1")
        })
    };
    let unit = |index: usize, from: &str, to: &str| {
        steps.get(index).is_some_and(|s| {
            s.name == "unitconvert"
                && !s.inverted
                && s.params.len() == 2
                && s.params[0].is("xy_in", from)
                && s.params[1].is("xy_out", to)
        })
    };
    let pop_at = |index: usize| {
        steps.get(index).is_some_and(|s| {
            s.name == "pop"
                && s.params.len() == 2
                && s.params[0].keyed("v_1")
                && s.params[1].keyed("v_2")
        })
    };
    let push_at = |index: usize| {
        steps.get(index).is_some_and(|s| {
            s.name == "push"
                && s.params.len() == 2
                && s.params[0].keyed("v_1")
                && s.params[1].keyed("v_2")
        })
    };
    // unitconvert rad->deg, axisswap, push, axisswap, unitconvert deg->rad, vgridshift,
    // unitconvert rad->deg, axisswap, pop: the whole wrapper goes, the vgridshift stays.
    if steps[prev].name == "unitconvert"
        && steps[prev].params.len() == 2
        && steps[prev].params[0].is("xy_in", "rad")
        && steps[prev].params[1].is("xy_out", "deg")
        && swap21(cur)
        && push_at(cur + 1)
        && swap21(cur + 2)
        && unit(cur + 3, "deg", "rad")
        && at(cur + 4, "vgridshift")
        && unit(cur + 5, "rad", "deg")
        && swap21(cur + 6)
        && pop_at(cur + 7)
    {
        steps.remove(cur + 7);
        steps.drain(prev..cur + 4);
        return true;
    }
    // axisswap, unitconvert deg->rad, vgridshift, unitconvert rad->deg, axisswap, push,
    // axisswap, unitconvert deg->rad: the push moves in front, the vgridshift follows.
    if swap21(prev)
        && unit(cur, "deg", "rad")
        && at(cur + 1, "vgridshift")
        && unit(cur + 2, "rad", "deg")
        && swap21(cur + 3)
        && push_at(cur + 4)
        && swap21(cur + 5)
        && unit(cur + 6, "deg", "rad")
    {
        let vgridshift = steps[cur + 1].clone();
        steps.drain(prev..cur + 4);
        steps.insert(prev + 3, vgridshift);
        return true;
    }
    false
}

fn flag(key: &str) -> Param {
    Param {
        key: key.to_owned(),
        value: String::new(),
    }
}

fn pattern_step(name: &str, flags: &[&str]) -> Step {
    Step {
        name: name.to_owned(),
        inverted: false,
        params: flags.iter().map(|key| flag(key)).collect(),
    }
}

fn adjacent_rule(steps: &mut Vec<Step>, cur: usize) -> Option<usize> {
    let prev = cur - 1;
    let len = steps.len();
    let is_last = cur + 1 == len;

    if !is_last
        && steps[cur].name == "longlat"
        && steps[cur].params.len() == 1
        && steps[cur].params[0].keyed("ellps")
    {
        steps.remove(cur);
        return Some(cur);
    }

    if let Some(next) = unitconvert_rewrite(steps, cur) {
        return Some(next);
    }

    let (p, c) = (&steps[prev], &steps[cur]);
    let plain = !p.inverted && !c.inverted;

    if c.name == "pop" && p.name == "push" && plain && one_param_same_key(p, c) {
        return Some(delete_pair(steps, prev));
    }
    if c.name == "push" && p.name == "pop" && plain && one_param_same_key(p, c) {
        return Some(delete_pair(steps, prev));
    }

    if !is_last
        && p.name == "unitconvert"
        && c.name == "axisswap"
        && is_order(c, "2,1")
        && steps[cur + 1].name == "unitconvert"
    {
        steps.swap(prev, cur);
        return Some(cur + 1);
    }

    if c.name == "axisswap" && p.name == "axisswap" && c.params.len() == 1 && p.params.len() == 1 {
        let (pp, cp) = (&p.params[0], &c.params[0]);
        if pp.is("order", "2,1") && cp.is("order", "2,1") {
            return Some(delete_pair(steps, prev));
        }
        if !p.inverted && !c.inverted && pp.is("order", "2,-1") && cp.is("order", "-2,1") {
            return Some(delete_pair(steps, prev));
        }
        if !p.inverted && !c.inverted && pp.is("order", "2,-1") && cp.is("order", "1,-2") {
            steps[prev].inverted = false;
            steps[prev].params[0].value = "2,1".to_owned();
            steps.remove(cur);
            return Some(cur);
        }
        let fold_to_1_minus_2 = (pp.is("order", "2,1") && !c.inverted && cp.is("order", "2,-1"))
            || (pp.is("order", "-2,1") && !p.inverted && cp.is("order", "2,1"));
        if fold_to_1_minus_2 {
            steps[prev].inverted = false;
            steps[prev].params[0].value = "1,-2".to_owned();
            steps.remove(cur);
            return Some(cur);
        }
    }

    if !is_last
        && p.name == "axisswap"
        && p.params.len() == 1
        && p.params[0].is("order", "2,1")
        && c.name == "unitconvert"
        && steps[cur + 1].name == "axisswap"
        && steps[cur + 1].params.len() == 1
        && steps[cur + 1].params[0].is("order", "2,1")
    {
        steps.remove(cur + 1);
        steps.remove(prev);
        return Some(back_up(prev));
    }

    // "set v_4=X" followed by "set v_4=X omit_fwd" is "set v_4=X"; "set v_4=X omit_inv" followed
    // by "set v_4=X" is "set v_4=X" (io.cpp:9461-9487).
    if plain && p.name == "set" && c.name == "set" && epoch_then_omit_fwd(p, c) {
        steps.remove(cur);
        return Some(cur);
    }
    if plain && p.name == "set" && c.name == "set" && omit_inv_then_epoch(p, c) {
        steps.remove(prev);
        return Some(prev);
    }

    if p.name == "cart"
        && c.name == "cart"
        && c.inverted != p.inverted
        && one_param_only(p)
        && one_param_only(c)
        && ((c.params[0].is("ellps", "WGS84") && p.params[0].is("ellps", "GRS80"))
            || (c.params[0].is("ellps", "GRS80") && p.params[0].is("ellps", "WGS84")))
    {
        return Some(delete_pair(steps, prev));
    }

    if p.name == "helmert" && c.name == "helmert" && p.params.len() == 3 && c.params.len() == 3 {
        if let Some(sum) = helmert_sum(p, c) {
            if sum == [0.0, 0.0, 0.0] {
                return Some(delete_pair(steps, prev));
            }
            let (left, values) = (&mut steps[prev], sum);
            left.inverted = false;
            for (index, (key, value)) in [("x", values[0]), ("y", values[1]), ("z", values[2])]
                .into_iter()
                .enumerate()
            {
                left.params[index] = Param {
                    key: key.to_owned(),
                    value: g15(value),
                };
            }
            steps.remove(cur);
            return Some(cur);
        }
    }

    if p.name == "helmert"
        && c.name == "helmert"
        && c.inverted == p.inverted
        && same_keys(p, c)
        && rotations_cancel_same_direction(p, c)
    {
        return Some(delete_pair(steps, prev));
    }

    if p.name == "helmert"
        && c.name == "helmert"
        && c.inverted != p.inverted
        && same_keys(p, c)
        && rotations_cancel_opposite_direction(p, c)
    {
        return Some(delete_pair(steps, prev));
    }

    if p.name == c.name && c.inverted != p.inverted && same_params(p, c) {
        return Some(delete_pair(steps, prev));
    }

    None
}

/// The `unitconvert` rules of `PROJStringFormatter::toString` for the pair
/// `(cur - 1, cur)`: a pair that cancels, or a pair folded into one step.
fn unitconvert_rewrite(steps: &mut Vec<Step>, cur: usize) -> Option<usize> {
    let prev = cur - 1;
    let (p, c) = (&steps[prev], &steps[cur]);
    if p.name != "unitconvert" || c.name != "unitconvert" || p.inverted || c.inverted {
        return None;
    }
    if xy_pair_cancels(p, c, "xy_in", "xy_out") || xy_pair_cancels(p, c, "z_in", "z_out") {
        return Some(delete_pair(steps, prev));
    }
    if p.params.len() == 4 && c.params.len() == 4 && xyz_pair_cancels(p, c) {
        return Some(delete_pair(steps, prev));
    }
    unitconvert_merge(steps, cur)
}

/// Folds the `xy` and `z` parts of two adjacent `unitconvert` steps into one,
/// or drops the `xy` part when it cancels. Each rule is one case of PROJ's
/// `unitconvert` merges in `PROJStringFormatter::toString`.
fn unitconvert_merge(steps: &mut Vec<Step>, cur: usize) -> Option<usize> {
    let prev = cur - 1;
    let (p, c) = (&steps[prev], &steps[cur]);
    let is_xy =
        |s: &Step| s.params.len() == 2 && s.params[0].keyed("xy_in") && s.params[1].keyed("xy_out");
    let is_z =
        |s: &Step| s.params.len() == 2 && s.params[0].keyed("z_in") && s.params[1].keyed("z_out");
    let is_xyz = |s: &Step| s.params.len() == 4 && unitconvert_xyz_keys(&s.params);
    let value = |s: &Step, index: usize| s.params[index].value.clone();
    let pair = |key: &str, value: String| Param {
        key: key.to_owned(),
        value,
    };

    if is_z(c) && is_xy(p) {
        let merged = vec![
            pair("xy_in", value(p, 0)),
            pair("z_in", value(c, 0)),
            pair("xy_out", value(p, 1)),
            pair("z_out", value(c, 1)),
        ];
        return Some(merge_into_cur(steps, prev, merged));
    }
    if is_z(p) && is_xy(c) {
        let merged = vec![
            pair("xy_in", value(c, 0)),
            pair("z_in", value(p, 0)),
            pair("xy_out", value(c, 1)),
            pair("z_out", value(p, 1)),
        ];
        return Some(merge_into_cur(steps, prev, merged));
    }

    if is_xyz(c) && is_xy(p) && value(c, 0) == value(p, 1) && value(c, 2) == value(p, 0) {
        return Some(keep_z_only(steps, prev, value(c, 1), value(c, 3)));
    }
    if is_xyz(p) && is_xy(c) && value(p, 0) == value(c, 1) && value(p, 2) == value(c, 0) {
        return Some(keep_z_only(steps, prev, value(p, 1), value(p, 3)));
    }

    if is_xyz(p) && is_z(c) && value(p, 3) == value(c, 0) {
        let merged = vec![
            pair("xy_in", value(p, 0)),
            pair("z_in", value(p, 1)),
            pair("xy_out", value(p, 2)),
            pair("z_out", value(c, 1)),
        ];
        return Some(merge_into_cur(steps, prev, merged));
    }
    if is_z(p) && is_xyz(c) && value(p, 1) == value(c, 1) {
        let merged = vec![
            pair("xy_in", value(c, 0)),
            pair("z_in", value(p, 0)),
            pair("xy_out", value(c, 2)),
            pair("z_out", value(c, 3)),
        ];
        return Some(merge_into_cur(steps, prev, merged));
    }
    if is_xyz(p) && is_xy(c) && value(p, 2) == value(c, 0) {
        let merged = vec![
            pair("xy_in", value(p, 0)),
            pair("z_in", value(p, 1)),
            pair("xy_out", value(c, 1)),
            pair("z_out", value(p, 3)),
        ];
        return Some(merge_into_cur(steps, prev, merged));
    }
    if is_xyz(p) && is_xyz(c) && value(p, 2) == value(c, 0) && value(c, 1) == value(c, 3) {
        let merged = vec![
            pair("xy_in", value(p, 0)),
            pair("z_in", value(p, 1)),
            pair("xy_out", value(c, 2)),
            pair("z_out", value(p, 3)),
        ];
        return Some(merge_into_cur(steps, prev, merged));
    }
    if is_xyz(p) && is_xyz(c) && value(p, 1) == value(p, 3) && value(c, 0) == value(p, 2) {
        let merged = vec![
            pair("xy_in", value(p, 0)),
            pair("z_in", value(c, 1)),
            pair("xy_out", value(c, 2)),
            pair("z_out", value(c, 3)),
        ];
        return Some(merge_into_cur(steps, prev, merged));
    }
    None
}

/// Replaces the parameters of `prev + 1` by `merged` and removes `prev`.
fn merge_into_cur(steps: &mut Vec<Step>, prev: usize, merged: Vec<Param>) -> usize {
    steps[prev + 1].params = merged;
    steps.remove(prev);
    back_up(prev)
}

/// A `z` conversion that remains once the `xy` part of the pair cancels.
fn keep_z_only(steps: &mut Vec<Step>, prev: usize, z_in: String, z_out: String) -> usize {
    if z_in == z_out {
        return delete_pair(steps, prev);
    }
    let merged = vec![
        Param {
            key: "z_in".to_owned(),
            value: z_in,
        },
        Param {
            key: "z_out".to_owned(),
            value: z_out,
        },
    ];
    merge_into_cur(steps, prev, merged)
}

/// Removes `prev` and `cur`. Returns the index to resume at, as PROJ does.
fn delete_pair(steps: &mut Vec<Step>, prev: usize) -> usize {
    steps.drain(prev..=prev + 1);
    back_up(prev)
}

fn back_up(position: usize) -> usize {
    position.saturating_sub(1).max(1)
}

fn one_param_only(step: &Step) -> bool {
    step.params.len() == 1
}

/// `set v_4=X` followed by `set v_4=X +omit_fwd` (io.cpp:9461-9473).
fn epoch_then_omit_fwd(p: &Step, c: &Step) -> bool {
    p.params.len() == 1
        && c.params.len() == 2
        && p.params[0].keyed("v_4")
        && c.params[0].keyed("v_4")
        && c.params[1].is("omit_fwd", "")
        && p.params[0].value == c.params[0].value
}

/// `set v_4=X +omit_inv` followed by `set v_4=X` (io.cpp:9475-9487).
fn omit_inv_then_epoch(p: &Step, c: &Step) -> bool {
    p.params.len() == 2
        && c.params.len() == 1
        && p.params[0].keyed("v_4")
        && c.params[0].keyed("v_4")
        && p.params[1].is("omit_inv", "")
        && p.params[0].value == c.params[0].value
}

fn one_param_same_key(p: &Step, c: &Step) -> bool {
    p.params.len() == 1 && c.params.len() == 1 && p.params[0].key == c.params[0].key
}

fn is_order(step: &Step, order: &str) -> bool {
    step.name == "axisswap" && step.params.len() == 1 && step.params[0].is("order", order)
}

fn xy_pair_cancels(p: &Step, c: &Step, in_key: &str, out_key: &str) -> bool {
    p.params.len() == 2
        && c.params.len() == 2
        && c.params[0].keyed(in_key)
        && p.params[0].keyed(in_key)
        && c.params[1].keyed(out_key)
        && p.params[1].keyed(out_key)
        && c.params[0].value == p.params[1].value
        && c.params[1].value == p.params[0].value
}

fn xyz_pair_cancels(p: &Step, c: &Step) -> bool {
    p.params[0].keyed("xy_in")
        && c.params[0].keyed("xy_in")
        && p.params[1].keyed("z_in")
        && c.params[1].keyed("z_in")
        && p.params[2].keyed("xy_out")
        && c.params[2].keyed("xy_out")
        && p.params[3].keyed("z_out")
        && c.params[3].keyed("z_out")
        && c.params[0].value == p.params[2].value
        && c.params[1].value == p.params[3].value
        && c.params[2].value == p.params[0].value
        && c.params[3].value == p.params[1].value
}

/// Sum of the translations of two three-parameter Helmert steps, with the
/// sign of each step's inversion. `None` when a translation is missing.
fn helmert_sum(p: &Step, c: &Step) -> Option<[f64; 3]> {
    let sign_p = if p.inverted { -1.0 } else { 1.0 };
    let sign_c = if c.inverted { -1.0 } else { 1.0 };
    let mut sum = [0.0; 3];
    for (slot, key) in ["x", "y", "z"].into_iter().enumerate() {
        let left = value_of(p, key)?;
        let right = value_of(c, key)?;
        sum[slot] = sign_p * left + sign_c * right;
    }
    Some(sum)
}

fn value_of(step: &Step, key: &str) -> Option<f64> {
    step.params
        .iter()
        .find(|param| param.keyed(key))
        .and_then(|param| param.value.parse::<f64>().ok())
}

fn same_keys(p: &Step, c: &Step) -> bool {
    p.params.len() == c.params.len()
        && p.params
            .iter()
            .all(|param| c.params.iter().any(|other| other.keyed(&param.key)))
}

fn same_params(p: &Step, c: &Step) -> bool {
    p.params.len() == c.params.len()
        && p.params
            .iter()
            .zip(&c.params)
            .all(|(left, right)| left == right)
}

/// Two Helmert steps with the same inversion cancel when the translations and
/// rotations are opposite and the convention and epochs agree.
fn rotations_cancel_same_direction(p: &Step, c: &Step) -> bool {
    p.params.iter().all(|param| {
        let other = c
            .params
            .iter()
            .find(|other| other.keyed(&param.key))
            .map(|other| other.value.as_str())
            .unwrap_or("");
        if matches!(param.key.as_str(), "convention" | "t_epoch" | "t_obs") {
            param.value == other
        } else {
            param.value.parse::<f64>().ok() == other.parse::<f64>().ok().map(|v| -v)
        }
    })
}

/// Two Helmert steps with opposite inversions cancel when the translations and
/// the other parameters agree and the rotations are opposite.
fn rotations_cancel_opposite_direction(p: &Step, c: &Step) -> bool {
    p.params.iter().all(|param| {
        let other = c
            .params
            .iter()
            .find(|other| other.keyed(&param.key))
            .map(|other| other.value.as_str())
            .unwrap_or("");
        match param.key.as_str() {
            "convention" => param.value != other,
            "rx" | "ry" | "rz" | "drx" | "dry" | "drz" => {
                param.value.parse::<f64>().ok() == other.parse::<f64>().ok().map(|v| -v)
            }
            _ => param.value == other,
        }
    })
}

/// `%.15g`, the text of a parameter that PROJ does not round to a tenth.
pub(crate) fn g15(value: f64) -> String {
    let text = format_g(value, 15);
    if text.contains("9999999999") {
        format_g(value, 14)
    } else {
        text
    }
}

/// A number as `PROJStringFormatter::addParam(name, double)` writes it: a value
/// within `1e-8` of a tenth is written as that tenth, then `%.15g`.
pub(crate) fn proj_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let scaled = value * 10.0;
    let nearest = scaled.round();
    let value = if (scaled - nearest).abs() < 1e-8 {
        nearest / 10.0
    } else {
        value
    };
    g15(value)
}
