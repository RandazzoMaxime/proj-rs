//! The PROJ pipeline text an operation exports: the steps `PROJStringFormatter`
//! collects, the inversion stack, and the rules its `toString()` applies
//! before printing.
//!
//! Not ported from `io.cpp`: the grid push/pop rewriting (`hgridshift`,
//! `vgridshift`, and the `omit_fwd`/`omit_inv` flags it sets), the Krovak
//! `axis=swu` merge, and the `set` epoch merge. Skipping one keeps the same
//! transform with more steps, so the text differs from PROJ on those inputs.
//! Values are written without quoting, which is enough for the numbers and
//! names an operation produces.

use super::wkt::format_g;

/// Line breaks in a PROJ string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjLines {
    /// One step per line, indented under `+proj=pipeline`.
    Multi,
    /// A single line.
    Single,
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
    /// `(official name, PROJ name)` of each grid the database renames.
    grid_names: Vec<(String, String)>,
    /// Whether the grid and deformation methods write their steps. Off, they are
    /// unimplemented, so the candidate selection of `cs2cs` is unchanged.
    grid_steps: bool,
}

impl PipelineFormatter {
    /// A formatter that writes each grid under its PROJ name, as
    /// `substitutePROJAlternativeGridNames` does with a database context.
    pub(crate) fn with_grid_names(grid_names: Vec<(String, String)>) -> Self {
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

    /// The name a grid is written under.
    pub(crate) fn grid_name(&self, name: &str) -> String {
        self.grid_names
            .iter()
            .find(|(original, _)| original == name)
            .map_or_else(|| name.to_owned(), |(_, proj)| proj.clone())
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
        }
        self.steps[start..].reverse();
    }

    pub(crate) fn set_optimize_units(&mut self) {
        self.optimize_units = true;
    }

    /// Runs the optimizations and prints the pipeline.
    pub(crate) fn finish(mut self, lines: ProjLines) -> String {
        self.drop_noops();
        self.normalize_inverses();
        self.merge_adjacent();
        self.render(lines)
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

    fn render(self, lines: ProjLines) -> String {
        let multi = lines == ProjLines::Multi;
        let mut result = String::new();
        let mut indent = 0;
        if self.steps.len() > 1 || self.steps.first().is_some_and(|step| step.inverted) {
            result.push_str("+proj=pipeline");
            if multi {
                indent += 1;
            }
        }
        for step in &self.steps {
            let mut line = if result.is_empty() {
                String::new()
            } else if multi {
                format!("{}+step", " ".repeat(indent * 2))
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
                if multi && line.len() + kv.len() > 80 {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&line);
                    line = " ".repeat(indent * 2 + "+step ".len());
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
        "set" => step.inverted = false,
        _ => {}
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
