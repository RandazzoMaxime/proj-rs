//! The libproj side of the bench, through `libproj_timer.c`: a separate program that build.rs
//! compiles against the libproj it finds. The bench sends it the points it computed and reads
//! back the times and the images, so no Rust code calls libproj (the repository has no `unsafe`).

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Whether a pipeline case first checks libproj on a UTM zone 31 point (ABI and definition).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UtmCheck {
    /// Check the easting of 3°E on the equator first.
    Check,
    /// No check.
    Skip,
}

/// The running timer program.
pub(crate) struct Timer {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Timer {
    /// Starts the program build.rs compiled.
    pub(crate) fn start() -> Self {
        let mut child = Command::new(env!("PROJ_RS_LIBPROJ_TIMER"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start the libproj timer");
        let input = child.stdin.take().expect("timer stdin");
        let output = BufReader::new(child.stdout.take().expect("timer stdout"));
        Self {
            child,
            input: Some(input),
            output,
        }
    }

    /// Sends a command line and its points, then reads the reply line: the numbers after `ok`.
    fn ask(&mut self, command: &str, points: impl Iterator<Item = (f64, f64)>) -> Vec<f64> {
        let mut bytes = Vec::new();
        writeln!(bytes, "{command}").expect("Vec write");
        for (x, y) in points {
            bytes.extend_from_slice(&x.to_ne_bytes());
            bytes.extend_from_slice(&y.to_ne_bytes());
        }
        let input = self.input.as_mut().expect("timer stdin");
        input.write_all(&bytes).expect("send to the timer");
        input.flush().expect("send to the timer");
        let mut line = String::new();
        self.output
            .read_line(&mut line)
            .expect("read the timer reply");
        let numbers = line
            .strip_prefix("ok ")
            .unwrap_or_else(|| panic!("libproj timer: {command}: {line}"));
        numbers
            .split_whitespace()
            .map(|number| number.parse().expect("a time"))
            .collect()
    }

    /// Reads `count` (x, y) pairs written back by the timer.
    fn points(&mut self, count: usize) -> Vec<(f64, f64)> {
        let mut bytes = vec![0_u8; count * 16];
        self.output
            .read_exact(&mut bytes)
            .expect("read the timer points");
        let value = |chunk: &[u8]| f64::from_ne_bytes(chunk.try_into().expect("eight bytes"));
        bytes
            .chunks_exact(16)
            .map(|pair| (value(&pair[..8]), value(&pair[8..])))
            .collect()
    }

    /// Nanoseconds per point of `proj_trans` with `proj_create(definition)` on `points`
    /// (radians, `t = 0`), after the UTM zone 31 check if asked.
    pub(crate) fn pipeline(
        &mut self,
        definition: &str,
        check: UtmCheck,
        points: &[(f64, f64)],
    ) -> f64 {
        let command = format!(
            "pipeline {} {} {definition}",
            points.len(),
            u8::from(check == UtmCheck::Check)
        );
        self.ask(&command, points.iter().copied())[0]
    }

    /// `proj_create_crs_to_crs` and `proj_normalize_for_visualization`, then nanoseconds per
    /// point of `proj_trans` one point at a time and of `proj_trans_generic` on the slice, and
    /// the images of the slice.
    pub(crate) fn facade(
        &mut self,
        from: &str,
        to: &str,
        points: &[(f64, f64)],
    ) -> (f64, f64, Vec<(f64, f64)>) {
        let command = format!("facade {} {from} {to}", points.len());
        let times = self.ask(&command, points.iter().copied());
        (times[0], times[1], self.points(points.len()))
    }

    /// Seconds for `total` points through `source`, one point at a time and as slices, and the
    /// images of the first slice.
    pub(crate) fn billion(
        &mut self,
        from: &str,
        to: &str,
        total: usize,
        source: &[(f64, f64)],
    ) -> (f64, f64, Vec<(f64, f64)>) {
        let command = format!("billion {} {total} {from} {to}", source.len());
        let times = self.ask(&command, source.iter().copied());
        (times[0], times[1], self.points(source.len()))
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        // Closing stdin ends the program's loop.
        drop(self.input.take());
        let _ = self.child.wait();
    }
}
