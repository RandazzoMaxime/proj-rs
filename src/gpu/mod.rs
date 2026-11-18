//! GPU back end for slice conversions (`wgpu`: Vulkan, Metal, DX12).
//!
//! [`Proj::convert_array_gpu`](crate::Proj::convert_array_gpu) runs on the GPU every transform
//! whose steps all have a GPU kernel, and sends every other one to the CPU path of
//! [`Proj::convert_array`](crate::Proj::convert_array), so a call never fails for want of a
//! kernel. Results agree with the CPU to a micrometre, see `docs/PORTING-gpu.md`.
//!
//! # Precision
//!
//! The GPU computes in `f64` where the device has `SHADER_F64` (Vulkan on NVIDIA and AMD), and in
//! double-float, two `f32` per number, everywhere else: Metal, DX12, Intel, mobile. Both agree
//! with the CPU to well under a millimetre.
//!
//! ```no_run
//! use proj_rs::gpu::GpuContext;
//! use proj_rs::Proj;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let gpu = GpuContext::new()?;
//! let proj = Proj::new_known_crs("EPSG:4326", "EPSG:32631", None)?;
//! let mut points = vec![(3.0_f64, 48.0_f64); 1_000_000];
//! let (_, report) = proj.convert_array_gpu(&mut points, &gpu)?;
//! println!("{} on the GPU, {} on the CPU", report.gpu_points, report.cpu_points);
//! # Ok(())
//! # }
//! ```

mod array;
mod device;
mod program;
mod shader;

pub use array::{Fallback, GpuReport};
pub use device::{GpuContext, GpuError, GpuOptions, Power, Precision};
pub(crate) use program::{GpuOp, GpuProgram, Opcode, Swap};
