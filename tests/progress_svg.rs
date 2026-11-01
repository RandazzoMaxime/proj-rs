//! `docs/progress.svg`: one square per PROJ test case, green when this port
//! passes it, grey when it is still open. Drawn from the same ledgers as the
//! README counts, so it cannot drift: `UPDATE_DOCS=1 cargo test --test
//! progress_svg` rewrites it, and the test fails when it is out of date.

#![allow(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "chart arithmetic on case counts far below 2^52"
)]

mod common;

use common::{cli_open, gie_files, unit_open, CLI_TOTALS, UNIT_TOTALS};
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

const BACKGROUND: &str = "#0d1117";
const DONE: &str = "#2ea043";
const OPEN: &str = "#6e7681";

struct Group {
    label: String,
    cases: usize,
    done: usize,
}

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Balanced binary treemap: split the weight in two halves, cut the longer side.
fn layout(groups: &[Group], order: &[usize], rect: Rect, out: &mut Vec<(usize, Rect)>) {
    if let [only] = order {
        out.push((*only, rect));
        return;
    }
    let total: usize = order.iter().map(|&i| groups[i].cases).sum();
    let mut running = 0;
    let mut cut = 1;
    for (position, &i) in order.iter().enumerate() {
        running += groups[i].cases;
        cut = position + 1;
        if running * 2 >= total {
            break;
        }
    }
    cut = cut.min(order.len() - 1);
    let head: usize = order[..cut].iter().map(|&i| groups[i].cases).sum();
    let share = head as f64 / total as f64;
    let (first, second) = if rect.w >= rect.h {
        let w = rect.w * share;
        (
            Rect { w, ..rect },
            Rect {
                x: rect.x + w,
                w: rect.w - w,
                ..rect
            },
        )
    } else {
        let h = rect.h * share;
        (
            Rect { h, ..rect },
            Rect {
                y: rect.y + h,
                h: rect.h - h,
                ..rect
            },
        )
    };
    layout(groups, &order[..cut], first, out);
    layout(groups, &order[cut..], second, out);
}

/// The square cells of one group, as two path strings: done and open.
fn cells(group: &Group, rect: Rect) -> (String, String) {
    let n = group.cases.max(1);
    let (count, ratio) = (n as f64, rect.w / rect.h);
    let cols = ((count * ratio).sqrt().round() as usize).clamp(1, n);
    let rows = n.div_ceil(cols);
    let (cw, ch) = (rect.w / cols as f64, rect.h / rows as f64);
    let (mut done, mut open) = (String::new(), String::new());
    for index in 0..group.cases {
        let (x, y) = (
            rect.x + (index % cols) as f64 * cw,
            rect.y + (index / cols) as f64 * ch,
        );
        let target = if index < group.done {
            &mut done
        } else {
            &mut open
        };
        let _ = write!(target, "M{x:.1} {y:.1}h{cw:.1}v{ch:.1}h-{cw:.1}z");
    }
    (done, open)
}

fn percent(done: usize, total: usize) -> String {
    let value = 100.0 * done as f64 / total.max(1) as f64;
    format!("{value:.1}%")
}

/// One panel: a header and the treemap of its groups.
fn panel(svg: &mut String, title: &str, groups: &[Group], x: f64, width: f64) {
    let done: usize = groups.iter().map(|g| g.done).sum();
    let total: usize = groups.iter().map(|g| g.cases).sum();
    let _ = writeln!(
        svg,
        "<text x=\"{:.2}\" y=\"21.00\" font-family=\"sans-serif\" font-size=\"16\" fill=\"#ffffff\" \
stroke=\"{BACKGROUND}\" stroke-width=\"3\" paint-order=\"stroke\">{title}: {} ({done}/{total})</text>",
        x + 4.0,
        percent(done, total)
    );
    let mut order: Vec<usize> = (0..groups.len()).collect();
    order.sort_by(|&a, &b| groups[b].cases.cmp(&groups[a].cases).then(a.cmp(&b)));
    let mut placed = Vec::new();
    layout(
        groups,
        &order,
        Rect {
            x: x + 1.0,
            y: 31.0,
            w: width - 2.0,
            h: 278.0,
        },
        &mut placed,
    );
    for (index, rect) in placed {
        let group = &groups[index];
        let inner = Rect {
            x: rect.x + 1.0,
            y: rect.y + 1.0,
            w: rect.w - 2.0,
            h: rect.h - 2.0,
        };
        let (done_path, open_path) = cells(group, inner);
        let _ = writeln!(
            svg,
            "<g><title>{}: {}/{} ({})</title>",
            group.label,
            group.done,
            group.cases,
            percent(group.done, group.cases)
        );
        for (path, colour) in [(done_path, DONE), (open_path, OPEN)] {
            if !path.is_empty() {
                let _ = writeln!(
                    svg,
                    "<path d=\"{path}\" fill=\"{colour}\" stroke=\"{BACKGROUND}\" stroke-width=\"0.5\"/>"
                );
            }
        }
        let _ = writeln!(
            svg,
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{BACKGROUND}\" stroke-width=\"2\"/>",
            rect.x, rect.y, rect.w, rect.h
        );
        let fit = ((rect.w - 8.0) / 6.6).max(0.0) as usize;
        if fit >= 2 && rect.h >= 16.0 {
            let label: String = group.label.chars().take(fit).collect();
            let _ = writeln!(
                svg,
                "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"sans-serif\" font-size=\"12\" fill=\"#ffffff\" \
stroke=\"{BACKGROUND}\" stroke-width=\"3\" paint-order=\"stroke\">{label}</text>",
                rect.x + 4.0,
                rect.y + 14.0
            );
        }
        svg.push_str("</g>\n");
    }
}

fn chart() -> String {
    let gie: Vec<Group> = gie_files()
        .into_iter()
        .map(|(name, cases, done)| Group {
            label: name.trim_end_matches(".gie").to_owned(),
            cases,
            done,
        })
        .collect();
    let (units, clis) = (unit_open(), cli_open());
    let mut api: Vec<Group> = UNIT_TOTALS
        .iter()
        .map(|(name, total)| Group {
            label: name
                .trim_start_matches("test_")
                .trim_end_matches(".cpp")
                .to_owned(),
            cases: *total,
            done: total - units.get(name).copied().unwrap_or(0),
        })
        .collect();
    api.extend(CLI_TOTALS.iter().map(|(name, total)| Group {
        label: format!("{name} CLI"),
        cases: *total,
        done: total - clis.get(name).copied().unwrap_or(0),
    }));
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1000 310\" width=\"1000\" height=\"310\">\n\
<rect x=\"0\" y=\"0\" width=\"1000\" height=\"310\" fill=\"{BACKGROUND}\"/>\n"
    );
    panel(&mut svg, "PROJ gie suite", &gie, 0.0, 496.0);
    panel(&mut svg, "PROJ unit and CLI cases", &api, 504.0, 496.0);
    svg.push_str("</svg>\n");
    svg
}

#[test]
fn progress_chart_is_current() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/progress.svg");
    let expected = chart();
    if std::env::var_os("UPDATE_DOCS").is_some() {
        fs::write(&path, &expected).expect("write progress.svg");
    } else {
        let current = fs::read_to_string(&path).unwrap_or_default();
        assert!(
            current == expected,
            "docs/progress.svg is out of date: run `UPDATE_DOCS=1 cargo test --test progress_svg`"
        );
    }
}
