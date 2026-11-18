//! The device: adapter choice, precision, the compiled interpreter, and the buffers of one
//! conversion.

use super::program::Encoded;
use super::shader;
use std::fmt;
use std::sync::mpsc;

/// The number format the shaders compute in.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Precision {
    /// Native `f64`: needs `SHADER_F64`, which Vulkan on NVIDIA and AMD has, Metal and DX12 do
    /// not.
    Float64,
    /// Double-float: two `f32` per number, about 48 bits of mantissa. Runs everywhere.
    DoubleFloat,
}

/// Which adapter to ask the system for.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Power {
    /// The discrete GPU when there is one.
    #[default]
    HighPerformance,
    /// The integrated GPU when there is one.
    LowPower,
}

/// How a [`GpuContext`] is made and when it is worth using.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct GpuOptions {
    /// `None` takes the best the device offers: `f64` where it has it, double-float otherwise.
    pub precision: Option<Precision>,
    /// Adapter preference.
    pub power: Power,
    /// A call with fewer points than this runs on the CPU: sending a few thousand points to a
    /// GPU costs more than computing them. The default is a conservative guess; the right value
    /// depends on the device and the transform, measure it with `proj-rs-bench`.
    pub min_points: usize,
    /// Points per dispatch. A call holds about `3 * 16 * chunk_points` bytes of device memory
    /// while it runs, and none after. Capped by the limits of the device.
    pub chunk_points: usize,
}

impl Default for GpuOptions {
    fn default() -> Self {
        Self {
            precision: None,
            power: Power::default(),
            min_points: 1 << 18,
            chunk_points: 1 << 20,
        }
    }
}

impl GpuOptions {
    /// Compute in `precision` instead of the best the device offers.
    #[must_use]
    pub fn with_precision(mut self, precision: Precision) -> Self {
        self.precision = Some(precision);
        self
    }

    /// Prefer the adapter of `power`.
    #[must_use]
    pub fn with_power(mut self, power: Power) -> Self {
        self.power = power;
        self
    }

    /// Calls under `min_points` run on the CPU. `0` sends every call to the GPU.
    #[must_use]
    pub fn with_min_points(mut self, min_points: usize) -> Self {
        self.min_points = min_points;
        self
    }

    /// Points per dispatch.
    #[must_use]
    pub fn with_chunk_points(mut self, chunk_points: usize) -> Self {
        self.chunk_points = chunk_points;
        self
    }
}

/// Why a [`GpuContext`] could not be made.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuError {
    /// No adapter: no GPU, no driver, or no software Vulkan.
    NoAdapter(String),
    /// The adapter refused the device.
    Device(String),
    /// [`Precision::Float64`] was asked and the adapter has no `SHADER_F64`.
    Float64Unavailable,
    /// The shader or its pipeline was refused, or double-float failed its self-check.
    Unusable(String),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAdapter(why) => write!(f, "no GPU adapter: {why}"),
            Self::Device(why) => write!(f, "the GPU device was refused: {why}"),
            Self::Float64Unavailable => f.write_str("the adapter has no 64-bit float in shaders"),
            Self::Unusable(why) => write!(f, "no usable GPU precision: {why}"),
        }
    }
}

impl std::error::Error for GpuError {}

/// A GPU device with the interpreter compiled for one precision.
///
/// Make one and keep it: creating it compiles the shader, which costs up to a second. It is
/// `Send + Sync`; calls from several threads share the device.
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    precision: Precision,
    options: GpuOptions,
    chunk_points: usize,
    adapter: String,
    backend: String,
}

impl fmt::Debug for GpuContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpuContext")
            .field("adapter", &self.adapter)
            .field("backend", &self.backend)
            .field("precision", &self.precision)
            .field("chunk_points", &self.chunk_points)
            .finish_non_exhaustive()
    }
}

/// Workgroup size of the interpreter, `@workgroup_size(64)`.
const GROUP: usize = 64;

/// What a block of points came to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Block {
    /// The block holds the results.
    Done,
    /// A point the GPU cannot guarantee: the block is untouched and the CPU takes it.
    Flagged,
}

impl GpuContext {
    /// The default adapter, [`GpuOptions::default`].
    ///
    /// # Errors
    ///
    /// [`GpuError`] when no adapter, device or precision works.
    pub fn new() -> Result<Self, GpuError> {
        Self::with_options(GpuOptions::default())
    }

    /// A context with `options`.
    ///
    /// # Errors
    ///
    /// [`GpuError`] when no adapter, device or precision works.
    pub fn with_options(options: GpuOptions) -> Result<Self, GpuError> {
        let instance = wgpu::Instance::default();
        let power_preference = match options.power {
            Power::HighPerformance => wgpu::PowerPreference::HighPerformance,
            Power::LowPower => wgpu::PowerPreference::LowPower,
        };
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference,
            ..Default::default()
        }))
        .map_err(|error| GpuError::NoAdapter(error.to_string()))?;
        let has_f64 = adapter.features().contains(wgpu::Features::SHADER_F64);
        if options.precision == Some(Precision::Float64) && !has_f64 {
            return Err(GpuError::Float64Unavailable);
        }
        let want_f64 = has_f64 && options.precision != Some(Precision::DoubleFloat);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("proj-rs"),
            required_features: if want_f64 {
                wgpu::Features::SHADER_F64
            } else {
                wgpu::Features::empty()
            },
            ..Default::default()
        }))
        .map_err(|error| GpuError::Device(error.to_string()))?;
        let info = adapter.get_info();
        let limits = device.limits();
        let chunk_points = options
            .chunk_points
            .min(usize::try_from(limits.max_storage_buffer_binding_size).unwrap_or(usize::MAX) / 16)
            .min(
                usize::try_from(limits.max_compute_workgroups_per_dimension).unwrap_or(usize::MAX)
                    * GROUP,
            )
            .max(GROUP);
        let order: &[Precision] = if want_f64 {
            &[Precision::Float64, Precision::DoubleFloat]
        } else {
            &[Precision::DoubleFloat]
        };
        let mut refused = Vec::new();
        for &precision in order {
            if options.precision.is_some_and(|asked| asked != precision) {
                continue;
            }
            match build(&device, &queue, precision) {
                Ok(pipeline) => {
                    return Ok(Self {
                        device,
                        queue,
                        pipeline,
                        precision,
                        options,
                        chunk_points,
                        adapter: info.name,
                        backend: format!("{:?}", info.backend),
                    });
                }
                Err(why) => refused.push(format!("{precision:?}: {why}")),
            }
        }
        Err(GpuError::Unusable(refused.join("; ")))
    }

    /// The precision the shaders compute in.
    #[must_use]
    pub fn precision(&self) -> Precision {
        self.precision
    }

    /// The adapter name the driver reports.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        &self.adapter
    }

    /// The graphics API in use: `Vulkan`, `Metal`, `Dx12`.
    #[must_use]
    pub fn backend(&self) -> &str {
        &self.backend
    }

    /// The options this context was made with.
    #[must_use]
    pub fn options(&self) -> &GpuOptions {
        &self.options
    }

    /// Points per dispatch after the limits of the device.
    #[must_use]
    pub fn chunk_points(&self) -> usize {
        self.chunk_points
    }

    /// The buffers and bindings of one conversion, for blocks of up to `capacity` points.
    pub(crate) fn session(
        &self,
        program: &Encoded,
        capacity: usize,
    ) -> Result<Session<'_>, String> {
        Session::new(self, program, capacity.min(self.chunk_points))
    }
}

/// The interpreter pipeline of `precision`. Double-float must first prove it still does exact
/// arithmetic on this driver.
fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    precision: Precision,
) -> Result<wgpu::ComputePipeline, String> {
    let source = shader::source(precision);
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("proj-rs interpreter"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let make = |entry: &str| {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(entry),
            layout: None,
            module: &module,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };
    let pipeline = make("main");
    let canary = (precision == Precision::DoubleFloat).then(|| make("canary"));
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.to_string());
    }
    if let Some(canary) = canary {
        if !double_float_exact(device, queue, &canary)? {
            return Err("two_sum or two_prod is not exact: the driver reassociates".to_owned());
        }
    }
    Ok(pipeline)
}

/// The self-check of the double-float: `two_sum(1, 2^-30)` must be `(1, 2^-30)` and
/// `two_prod(1 + 2^-12, 1 + 2^-12)` must be `(1 + 2^-11, 2^-24)`. A driver that reassociates
/// floating-point operations (fast-math) returns zero for the error terms.
fn double_float_exact(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    canary: &wgpu::ComputePipeline,
) -> Result<bool, String> {
    let program = Encoded {
        instructions: vec![[u32::MAX, 0, 0, 0]],
        constants: vec![0.0],
    };
    let session = Session::raw(device, queue, canary, Precision::DoubleFloat, &program, 2)?;
    let input: [[f32; 4]; 2] = [[1.0, 2f32.powi(-30), 1.0 + 2f32.powi(-12), 0.0], [0.0; 4]];
    let words: &[u8] = bytemuck::cast_slice(&input);
    let out = session.dispatch_raw(words, 1, 2)?;
    let got: &[[f32; 4]] = bytemuck::cast_slice(&out[..32]);
    let want = [1.0, 2f32.powi(-30), 1.0 + 2f32.powi(-11), 2f32.powi(-24)];
    Ok(got[0] == want)
}

/// The buffers and the bind group of one conversion.
pub(crate) struct Session<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    pipeline: &'a wgpu::ComputePipeline,
    precision: Precision,
    instructions: u32,
    capacity: usize,
    input: wgpu::Buffer,
    output: wgpu::Buffer,
    status: wgpu::Buffer,
    count: wgpu::Buffer,
    staging: wgpu::Buffer,
    status_staging: wgpu::Buffer,
    bind: wgpu::BindGroup,
    scratch: Vec<[f32; 4]>,
}

fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn buffer(device: &wgpu::Device, size: usize, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::try_from(size).unwrap_or(u64::MAX),
        usage,
        mapped_at_creation: false,
    })
}

impl<'a> Session<'a> {
    fn new(context: &'a GpuContext, program: &Encoded, capacity: usize) -> Result<Self, String> {
        Self::raw(
            &context.device,
            &context.queue,
            &context.pipeline,
            context.precision,
            program,
            capacity,
        )
    }

    fn raw(
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        pipeline: &'a wgpu::ComputePipeline,
        precision: Precision,
        program: &Encoded,
        capacity: usize,
    ) -> Result<Self, String> {
        let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let bytes = capacity.max(1) * 16;
        let constants = match precision {
            Precision::Float64 => bytemuck::cast_slice::<f64, u8>(&program.constants).to_vec(),
            Precision::DoubleFloat => {
                let pairs: Vec<[f32; 2]> = program
                    .constants
                    .iter()
                    .map(|&value| {
                        let (hi, lo) = shader::split(value);
                        [hi, lo]
                    })
                    .collect();
                bytemuck::cast_slice::<[f32; 2], u8>(&pairs).to_vec()
            }
        };
        let params = buffer(device, constants.len().max(16), storage);
        queue.write_buffer(&params, 0, &constants);
        let instructions = buffer(device, program.instructions.len().max(1) * 16, storage);
        queue.write_buffer(
            &instructions,
            0,
            bytemuck::cast_slice(&program.instructions),
        );
        let input = buffer(device, bytes, storage);
        let output = buffer(
            device,
            bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let status = buffer(
            device,
            16,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        );
        let count = buffer(
            device,
            16,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let read = wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST;
        let staging = buffer(device, bytes, read);
        let status_staging = buffer(device, 16, read);
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                entry(0, &params),
                entry(1, &input),
                entry(2, &output),
                entry(3, &count),
                entry(4, &instructions),
                entry(5, &status),
            ],
        });
        let validation_error = pollster::block_on(validation.pop());
        let memory_error = pollster::block_on(memory.pop());
        if let Some(error) = validation_error.or(memory_error) {
            return Err(error.to_string());
        }
        Ok(Self {
            device,
            queue,
            pipeline,
            precision,
            instructions: u32::try_from(program.instructions.len()).map_err(|e| e.to_string())?,
            capacity,
            input,
            output,
            status,
            count,
            staging,
            status_staging,
            bind,
            scratch: Vec::new(),
        })
    }

    /// The block through the program. [`Block::Flagged`] leaves `block` as it was.
    pub(crate) fn run(&mut self, block: &mut [[f64; 2]]) -> Result<Block, String> {
        if block.len() > self.capacity {
            return Err("block larger than the session".to_owned());
        }
        if self.precision == Precision::DoubleFloat {
            self.scratch.clear();
            self.scratch.extend(block.iter().map(|&[a, b]| {
                let (a, b) = (shader::split(a), shader::split(b));
                [a.0, a.1, b.0, b.1]
            }));
        }
        let input: &[u8] = match self.precision {
            Precision::Float64 => bytemuck::cast_slice(block),
            Precision::DoubleFloat => bytemuck::cast_slice(&self.scratch),
        };
        let raw = self.dispatch_raw(input, self.instructions, block.len())?;
        let Some(raw) = raw_if_clean(&raw) else {
            return Ok(Block::Flagged);
        };
        match self.precision {
            Precision::Float64 => {
                let results: &[[f64; 2]] = bytemuck::cast_slice(raw);
                block.copy_from_slice(&results[..block.len()]);
            }
            Precision::DoubleFloat => {
                let results: &[[f32; 4]] = bytemuck::cast_slice(raw);
                for (slot, result) in block.iter_mut().zip(results) {
                    *slot = [
                        f64::from(result[0]) + f64::from(result[1]),
                        f64::from(result[2]) + f64::from(result[3]),
                    ];
                }
            }
        }
        Ok(Block::Done)
    }

    /// Upload `input` (16 bytes per point), run the program on `points` points, and read the
    /// output back with the status word appended: the first `16 * points` bytes are the
    /// output, the last 4 are the failure flag.
    fn dispatch_raw(
        &self,
        input: &[u8],
        instructions: u32,
        points: usize,
    ) -> Result<Vec<u8>, String> {
        let bytes = points * 16;
        let memory = self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        self.queue.write_buffer(&self.input, 0, input);
        let header = [
            u32::try_from(points).map_err(|e| e.to_string())?,
            u32::MAX,
            instructions,
            0,
        ];
        self.queue
            .write_buffer(&self.count, 0, bytemuck::cast_slice(&header));
        self.queue
            .write_buffer(&self.status, 0, bytemuck::cast_slice(&[0u32; 4]));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            let groups = u32::try_from(points.div_ceil(GROUP)).map_err(|e| e.to_string())?;
            pass.dispatch_workgroups(groups, 1, 1);
        }
        let byte_count = u64::try_from(bytes).map_err(|e| e.to_string())?;
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.staging, 0, byte_count);
        encoder.copy_buffer_to_buffer(&self.status, 0, &self.status_staging, 0, 16);
        self.queue.submit([encoder.finish()]);
        let (data_tx, data_rx) = mpsc::channel();
        let (flag_tx, flag_rx) = mpsc::channel();
        self.staging
            .slice(..byte_count)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = data_tx.send(result);
            });
        self.status_staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = flag_tx.send(result);
            });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| error.to_string())?;
        let validation_error = pollster::block_on(validation.pop());
        let memory_error = pollster::block_on(memory.pop());
        if let Some(error) = validation_error.or(memory_error) {
            return Err(error.to_string());
        }
        data_rx
            .recv()
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        flag_rx
            .recv()
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        let mut out = Vec::with_capacity(bytes + 4);
        {
            let view = self
                .staging
                .slice(..byte_count)
                .get_mapped_range()
                .map_err(|error| error.to_string())?;
            out.extend_from_slice(&view);
        }
        self.staging.unmap();
        {
            let view = self
                .status_staging
                .slice(..)
                .get_mapped_range()
                .map_err(|error| error.to_string())?;
            out.extend_from_slice(&view[..4]);
        }
        self.status_staging.unmap();
        Ok(out)
    }
}

/// The output bytes of [`Session::dispatch_raw`] without the status word, or `None` when the
/// status says a point was flagged.
fn raw_if_clean(raw: &[u8]) -> Option<&[u8]> {
    let (data, status) = raw.split_at(raw.len() - 4);
    let flag = u32::from_le_bytes(status.try_into().ok()?);
    (flag == 0).then_some(data)
}
