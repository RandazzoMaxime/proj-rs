//! The program the GPU runs for one transform: a list of small operations on the pair (a, b),
//! and the constants they read. The same list serves every transform; the shader is built once.
//!
//! A step lowers itself to operations with `Step::lower_gpu` (`engine.rs`). A step that cannot
//! says so by returning `false`, and the transform then runs on the CPU.

/// One operation of the interpreter. The discriminant is the opcode `shader.rs` writes into the
/// WGSL as `OP_<NAME>`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Opcode {
    /// `out_a = in_a (or in_b when swapped) * m0`, `out_b = in_b (or in_a) * m1`.
    Affine,
    /// `fwd_prepare`: longitude wrap and `lon_0` removal. Constants: the offset.
    PrepFwd,
    /// `fwd_finalize`: scale and false origin. Constants: scale, x0, y0.
    FinishFwd,
    /// Inverse prepare: unit, false origin and `1/a`. Constants: to_meter, x0, y0, ra.
    PrepInv,
    /// Inverse finalize: `lon_0` back, longitude wrap. Constants: the offset.
    FinishInv,
    /// Spherical Mercator, forward. Constants: k0.
    MercFwd,
    /// Spherical Mercator, inverse. Constants: k0.
    MercInv,
    /// Exact transverse Mercator, forward: qn, zb and the four Fourier tables.
    TmercFwd,
    /// Exact transverse Mercator, inverse.
    TmercInv,
    /// Lambert conformal conic, forward: e, n, c, rho0, k0.
    LccFwd,
    /// Lambert conformal conic, inverse.
    LccInv,
}

impl Opcode {
    /// Every opcode with the name it has in the WGSL.
    pub(crate) const ALL: [(Self, &'static str); 11] = [
        (Self::Affine, "OP_AFFINE"),
        (Self::PrepFwd, "OP_PREP_FWD"),
        (Self::FinishFwd, "OP_FINISH_FWD"),
        (Self::PrepInv, "OP_PREP_INV"),
        (Self::FinishInv, "OP_FINISH_INV"),
        (Self::MercFwd, "OP_MERC_FWD"),
        (Self::MercInv, "OP_MERC_INV"),
        (Self::TmercFwd, "OP_TMERC_FWD"),
        (Self::TmercInv, "OP_TMERC_INV"),
        (Self::LccFwd, "OP_LCC_FWD"),
        (Self::LccInv, "OP_LCC_INV"),
    ];

    pub(crate) fn code(self) -> u32 {
        match self {
            Self::Affine => 0,
            Self::PrepFwd => 1,
            Self::FinishFwd => 2,
            Self::PrepInv => 3,
            Self::FinishInv => 4,
            Self::MercFwd => 5,
            Self::MercInv => 6,
            Self::TmercFwd => 7,
            Self::TmercInv => 8,
            Self::LccFwd => 9,
            Self::LccInv => 10,
        }
    }
}

/// Whether an [`Opcode::Affine`] exchanges its two axes first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Swap {
    /// `a` stays `a`.
    No,
    /// The first output reads the second input.
    Yes,
}

/// One operation and its constants.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GpuOp {
    code: Opcode,
    swap: Swap,
    constants: Vec<f64>,
}

impl GpuOp {
    pub(crate) fn new(code: Opcode, constants: Vec<f64>) -> Self {
        Self {
            code,
            swap: Swap::No,
            constants,
        }
    }

    /// A signed, scaled permutation of the two horizontal axes.
    pub(crate) fn affine(swap: Swap, mx: f64, my: f64) -> Self {
        Self {
            code: Opcode::Affine,
            swap,
            constants: vec![mx, my],
        }
    }
}

/// The operations of one transform, in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct GpuProgram {
    ops: Vec<GpuOp>,
}

/// A program ready for the device: the instructions and the constant table.
#[derive(Clone, Debug)]
pub(crate) struct Encoded {
    /// `[opcode, first constant, flags, 0]` per operation.
    pub(crate) instructions: Vec<[u32; 4]>,
    /// The constants of every operation, in order.
    pub(crate) constants: Vec<f64>,
}

impl GpuProgram {
    pub(crate) fn push(&mut self, op: GpuOp) {
        self.ops.push(op);
    }

    /// `true` when the program holds at least one operation that is not an affine one: a program
    /// of axis swaps alone is not worth a trip to the GPU.
    pub(crate) fn has_projection(&self) -> bool {
        self.ops.iter().any(|op| op.code != Opcode::Affine)
    }

    /// The instruction words and the constant table.
    ///
    /// # Panics
    ///
    /// Panics when the constant table holds more than `u32::MAX` values, which no transform
    /// comes near.
    pub(crate) fn encode(&self) -> Encoded {
        let mut instructions = Vec::with_capacity(self.ops.len());
        let mut constants = Vec::new();
        for op in &self.ops {
            let base = u32::try_from(constants.len()).expect("constant table");
            let flags = u32::from(op.swap == Swap::Yes);
            instructions.push([op.code.code(), base, flags, 0]);
            constants.extend_from_slice(&op.constants);
        }
        Encoded {
            instructions,
            constants,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_lays_the_constants_end_to_end() {
        let mut program = GpuProgram::default();
        program.push(GpuOp::affine(Swap::Yes, 2.0, 3.0));
        program.push(GpuOp::new(Opcode::MercFwd, vec![0.9996]));
        let encoded = program.encode();
        assert_eq!(encoded.instructions, vec![[0, 0, 1, 0], [5, 2, 0, 0]]);
        assert_eq!(encoded.constants, vec![2.0, 3.0, 0.9996]);
        assert!(program.has_projection());
    }

    #[test]
    fn opcodes_are_distinct_and_in_order() {
        for (index, (opcode, _)) in Opcode::ALL.iter().enumerate() {
            assert_eq!(usize::try_from(opcode.code()).ok(), Some(index));
        }
    }
}
