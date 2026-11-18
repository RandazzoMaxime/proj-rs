//! Spike GPU : Web Mercator et UTM en WGSL, en `f64` natif et en double-float (deux f32).
//!
//! Précision contre proj-rs (PROJ 9.9.0), débit du noyau seul, débit de bout en bout (envoi,
//! calcul, lecture), et les mêmes points sur le CPU. Tout sort en Markdown sur stdout, le suivi
//! sur stderr. Voir README.md.

mod auxlat;
mod shaders;

use auxlat::{auxlat_coeffs, rectifying_radius, AuxLat};
use proj_rs::Proj;
use shaders::Path;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::time::Instant;

const WGS84_A: f64 = 6_378_137.0;
const WGS84_RF: f64 = 298.257_223_563;
const UTM_ZONE: u32 = 31;
/// Points par envoi : 4 M x 16 octets = 64 Mio, sous les 128 Mio de `maxStorageBufferBindingSize`.
const CHUNK: usize = 4_000_000;
const GROUP: u32 = 64;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Kernel {
    MercFwd,
    MercInv,
    UtmFwd,
    UtmInv,
}

const KERNELS: [Kernel; 4] = [
    Kernel::MercFwd,
    Kernel::MercInv,
    Kernel::UtmFwd,
    Kernel::UtmInv,
];

impl Kernel {
    fn entry(self) -> &'static str {
        match self {
            Self::MercFwd => "merc_fwd",
            Self::MercInv => "merc_inv",
            Self::UtmFwd => "utm_fwd",
            Self::UtmInv => "utm_inv",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::MercFwd => "4326 -> 3857",
            Self::MercInv => "3857 -> 4326",
            Self::UtmFwd => "4326 -> 32631 (UTM 31N)",
            Self::UtmInv => "32631 -> 4326",
        }
    }
    fn inverse(self) -> bool {
        matches!(self, Self::MercInv | Self::UtmInv)
    }
    fn crs(self) -> (&'static str, &'static str) {
        match self {
            Self::MercFwd => ("EPSG:4326", "EPSG:3857"),
            Self::MercInv => ("EPSG:3857", "EPSG:4326"),
            Self::UtmFwd => ("EPSG:4326", "EPSG:32631"),
            Self::UtmInv => ("EPSG:32631", "EPSG:4326"),
        }
    }
    /// Boîte (lon0, lon1, lat0, lat1) des points tirés, en degrés.
    fn area(self) -> (f64, f64, f64, f64) {
        match self {
            Self::MercFwd | Self::MercInv => (-180.0, 180.0, -85.0, 85.0),
            Self::UtmFwd | Self::UtmInv => (0.0, 6.0, -80.0, 84.0),
        }
    }
}

// ---------------------------------------------------------------- paramètres

/// Constantes du buffer `P`, dans l'ordre des `P_*` de kernels.wgsl.
fn params() -> Vec<f64> {
    let f = 1.0 / WGS84_RF;
    let n = f / (2.0 - f);
    let k0 = 0.9996;
    let coeffs = |auxin, auxout| auxlat_coeffs(n, auxin, auxout).expect("table générée");
    let cbg = coeffs(AuxLat::Geographic, AuxLat::Conformal);
    let cgb = coeffs(AuxLat::Conformal, AuxLat::Geographic);
    let utg = coeffs(AuxLat::Rectifying, AuxLat::Conformal);
    let gtu = coeffs(AuxLat::Conformal, AuxLat::Rectifying);
    let mut p = vec![
        WGS84_A,
        k0 * WGS84_A * rectifying_radius(n), // qn en mètres
        0.0,                                 // zb : lat_0 = 0 pour UTM
        500_000.0,
        0.0, // faux nord, hémisphère nord
        (f64::from(UTM_ZONE) - 30.5) * 6.0,
        std::f64::consts::PI / 180.0,
        180.0 / std::f64::consts::PI,
    ];
    for table in [cbg, cgb, gtu, utg] {
        p.extend_from_slice(&table);
    }
    assert_eq!(p.len(), 32);
    p
}

fn split(v: f64) -> [f32; 2] {
    #[expect(clippy::cast_possible_truncation, reason = "découpage hi/lo voulu")]
    let hi = v as f32;
    #[expect(clippy::cast_possible_truncation, reason = "découpage hi/lo voulu")]
    let lo = (v - f64::from(hi)) as f32;
    [hi, lo]
}

fn join(hi: f32, lo: f32) -> f64 {
    f64::from(hi) + f64::from(lo)
}

/// Un point sur le GPU : 16 octets. df : quatre f32 (hi, lo, hi, lo). f64 : deux f64.
type Raw = [u64; 2];

fn pack(path: Path, points: &[(f64, f64)]) -> Vec<Raw> {
    points
        .iter()
        .map(|&(a, b)| match path {
            Path::Df => {
                let (a, b) = (split(a), split(b));
                bytemuck::cast([a[0], a[1], b[0], b[1]])
            }
            Path::F64 => [a.to_bits(), b.to_bits()],
        })
        .collect()
}

fn unpack(path: Path, raw: &[Raw]) -> Vec<(f64, f64)> {
    raw.iter()
        .map(|&v| match path {
            Path::Df => {
                let v: [f32; 4] = bytemuck::cast(v);
                (join(v[0], v[1]), join(v[2], v[3]))
            }
            Path::F64 => (f64::from_bits(v[0]), f64::from_bits(v[1])),
        })
        .collect()
}

// ---------------------------------------------------------------- GPU

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    params: HashMap<Path, wgpu::Buffer>,
    pipes: HashMap<(Path, Kernel), wgpu::ComputePipeline>,
    canary: Option<wgpu::ComputePipeline>,
    paths: Vec<Path>,
    notes: Vec<String>,
}

struct Work {
    input: wgpu::Buffer,
    output: wgpu::Buffer,
    staging: wgpu::Buffer,
    count: wgpu::Buffer,
}

fn storage(device: &wgpu::Device, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage,
        mapped_at_creation: false,
    })
}

impl Gpu {
    async fn new(adapter: &wgpu::Adapter) -> Result<Self, String> {
        let has_f64 = adapter.features().contains(wgpu::Features::SHADER_F64);
        let required_features = if has_f64 {
            wgpu::Features::SHADER_F64
        } else {
            wgpu::Features::empty()
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("request_device: {e}"))?;
        let mut paths = vec![Path::Df];
        if has_f64 {
            paths.push(Path::F64);
        }
        let mut notes = Vec::new();
        if !has_f64 {
            notes.push("SHADER_F64 absent sur cet adaptateur : chemin f64 non testé.".to_owned());
        }
        let raw = params();
        let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let pairs: Vec<[f32; 2]> = raw.iter().map(|&v| split(v)).collect();
        let df_buffer = storage(&device, (pairs.len() * 8) as u64, usage);
        queue.write_buffer(&df_buffer, 0, bytemuck::cast_slice(&pairs));
        let f64_buffer = storage(&device, (raw.len() * 8) as u64, usage);
        queue.write_buffer(&f64_buffer, 0, bytemuck::cast_slice(&raw));
        let params = HashMap::from([(Path::Df, df_buffer), (Path::F64, f64_buffer)]);
        let mut gpu = Self {
            device,
            queue,
            params,
            pipes: HashMap::new(),
            canary: None,
            paths: Vec::new(),
            notes,
        };
        for path in paths {
            match gpu.build(path).await {
                Ok(()) => gpu.paths.push(path),
                Err(e) => gpu
                    .notes
                    .push(format!("chemin {} refusé : {e}", path.name())),
            }
        }
        Ok(gpu)
    }

    async fn build(&mut self, path: Path) -> Result<(), String> {
        let source = shaders::source(path);
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(path.name()),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let mut made = Vec::new();
        for kernel in KERNELS {
            made.push((kernel, self.pipeline(&module, kernel.entry())));
        }
        let canary = (path == Path::Df).then(|| self.pipeline(&module, "canary"));
        if let Some(error) = scope.pop().await {
            return Err(error.to_string());
        }
        for (kernel, pipe) in made {
            self.pipes.insert((path, kernel), pipe);
        }
        if canary.is_some() {
            self.canary = canary;
        }
        Ok(())
    }

    fn pipeline(&self, module: &wgpu::ShaderModule, entry: &str) -> wgpu::ComputePipeline {
        self.device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
    }

    fn work(&self, points: usize) -> Work {
        let size = (points.max(1) * 16) as u64;
        Work {
            input: storage(
                &self.device,
                size,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
            output: storage(
                &self.device,
                size,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            ),
            staging: storage(
                &self.device,
                size,
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            ),
            count: storage(
                &self.device,
                16,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
        }
    }

    fn bind(&self, path: Path, pipe: &wgpu::ComputePipeline, work: &Work) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipe.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params[&path].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: work.input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: work.output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: work.count.as_entire_binding(),
                },
            ],
        })
    }

    fn wait(&self) {
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
    }

    fn upload(&self, work: &Work, chunk: &[Raw]) {
        self.queue
            .write_buffer(&work.input, 0, bytemuck::cast_slice(chunk));
        // y = masque de la barrière du chemin df (voir `bar` dans prelude_df.wgsl).
        let count = [u32::try_from(chunk.len()).expect("chunk"), u32::MAX, 0, 0];
        self.queue
            .write_buffer(&work.count, 0, bytemuck::cast_slice(&count));
    }

    /// `reps` envois du même noyau dans une seule passe ; secondes au total.
    fn compute(
        &self,
        path: Path,
        pipe: &wgpu::ComputePipeline,
        work: &Work,
        n: usize,
        reps: u32,
    ) -> f64 {
        let bind = self.bind(path, pipe, work);
        let groups = u32::try_from(n).expect("n").div_ceil(GROUP);
        let start = Instant::now();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, &bind, &[]);
            for _ in 0..reps {
                pass.dispatch_workgroups(groups, 1, 1);
            }
        }
        self.queue.submit([encoder.finish()]);
        self.wait();
        start.elapsed().as_secs_f64()
    }

    fn download(&self, work: &Work, n: usize, out: &mut Vec<Raw>) {
        let bytes = (n * 16) as u64;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&work.output, 0, &work.staging, 0, bytes);
        self.queue.submit([encoder.finish()]);
        let slice = work.staging.slice(..bytes);
        slice.map_async(wgpu::MapMode::Read, |result| {
            result.expect("map_async");
        });
        self.wait();
        {
            let view = slice.get_mapped_range().expect("mapped range");
            out.extend_from_slice(bytemuck::cast_slice(&view));
        }
        work.staging.unmap();
    }

    /// Envoi, calcul et lecture par blocs de [`CHUNK`] points.
    fn process(
        &self,
        path: Path,
        pipe: &wgpu::ComputePipeline,
        work: &Work,
        input: &[Raw],
        out: &mut Vec<Raw>,
    ) {
        out.clear();
        for chunk in input.chunks(CHUNK) {
            self.upload(work, chunk);
            self.compute(path, pipe, work, chunk.len(), 1);
            self.download(work, chunk.len(), out);
        }
    }

    /// Le test de contrôle du double-float : `true` si two_sum et two_prod sont exacts.
    fn canary_ok(&self) -> Option<(bool, [f32; 4])> {
        let pipe = self.canary.as_ref()?;
        let work = self.work(1);
        let input: [Raw; 1] = [bytemuck::cast([
            1.0f32,
            2f32.powi(-30),
            1.0 + 2f32.powi(-12),
            0.0,
        ])];
        self.upload(&work, &input);
        let bind = self.bind(Path::Df, pipe, &work);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        self.queue.submit([encoder.finish()]);
        self.wait();
        let mut out = Vec::new();
        self.download(&work, 1, &mut out);
        let got: [f32; 4] = bytemuck::cast(out[0]);
        let want = [1.0, 2f32.powi(-30), 1.0 + 2f32.powi(-11), 2f32.powi(-24)];
        Some((got == want, got))
    }
}

// ---------------------------------------------------------------- données

fn lcg(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    #[expect(clippy::cast_precision_loss, reason = "53 bits utiles")]
    {
        (*state >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn geographic_points(kernel: Kernel, n: usize, seed: u64) -> Vec<(f64, f64)> {
    let (lon0, lon1, lat0, lat1) = kernel.area();
    let mut state = seed;
    (0..n)
        .map(|_| {
            let lon = lon0 + (lon1 - lon0) * lcg(&mut state);
            let lat = lat0 + (lat1 - lat0) * lcg(&mut state);
            (lon, lat)
        })
        .collect()
}

fn proj(from: &str, to: &str) -> Proj {
    Proj::new_known_crs(from, to, None).expect("proj-rs crée la paire")
}

/// Entrées du noyau et sortie de référence de proj-rs.
fn reference(kernel: Kernel, n: usize) -> (Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let geographic = geographic_points(kernel, n, 0x5eed);
    let forward = match kernel {
        Kernel::MercFwd | Kernel::MercInv => proj("EPSG:4326", kernel_projected(kernel)),
        Kernel::UtmFwd | Kernel::UtmInv => proj("EPSG:4326", kernel_projected(kernel)),
    };
    let projected: Vec<(f64, f64)> = geographic
        .iter()
        .map(|&p| forward.convert(p).expect("point dans le domaine"))
        .collect();
    if kernel.inverse() {
        let (from, to) = kernel.crs();
        let back = proj(from, to);
        let expected: Vec<(f64, f64)> = projected
            .iter()
            .map(|&p| back.convert(p).expect("retour"))
            .collect();
        (projected, expected, geographic)
    } else {
        (geographic.clone(), projected, geographic)
    }
}

fn kernel_projected(kernel: Kernel) -> &'static str {
    match kernel {
        Kernel::MercFwd | Kernel::MercInv => "EPSG:3857",
        Kernel::UtmFwd | Kernel::UtmInv => "EPSG:32631",
    }
}

// ---------------------------------------------------------------- mesures

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

struct Accuracy {
    max_mm: f64,
    mean_mm: f64,
    over_1mm: usize,
    bad: usize,
}

fn error_mm(kernel: Kernel, got: (f64, f64), want: (f64, f64), at_lat: f64) -> f64 {
    if kernel.inverse() {
        let dy = (got.1 - want.1) * 111_132.9 * 1000.0;
        let dx = (got.0 - want.0) * 111_319.5 * at_lat.to_radians().cos() * 1000.0;
        dx.hypot(dy)
    } else {
        ((got.0 - want.0) * 1000.0).hypot((got.1 - want.1) * 1000.0)
    }
}

fn accuracy(
    kernel: Kernel,
    got: &[(f64, f64)],
    want: &[(f64, f64)],
    geographic: &[(f64, f64)],
) -> Accuracy {
    let (mut max_mm, mut sum, mut over, mut bad) = (0.0f64, 0.0, 0, 0);
    for ((&g, &w), &geo) in got.iter().zip(want).zip(geographic) {
        let e = error_mm(kernel, g, w, geo.1);
        if !e.is_finite() {
            bad += 1;
            continue;
        }
        max_mm = max_mm.max(e);
        sum += e;
        if e > 1.0 {
            over += 1;
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "n < 2^53")]
    let mean_mm = sum / got.len().max(1) as f64;
    Accuracy {
        max_mm,
        mean_mm,
        over_1mm: over,
        bad,
    }
}

fn ns(seconds: f64, points: usize) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "n < 2^53")]
    {
        seconds * 1e9 / points as f64
    }
}

struct Options {
    sizes: Vec<usize>,
    accuracy_points: usize,
    adapter: Option<String>,
    cpu_points: usize,
    debug: bool,
}

fn parse_args() -> Options {
    let mut options = Options {
        sizes: vec![100_000, 1_000_000, 4_000_000],
        accuracy_points: 200_000,
        adapter: None,
        cpu_points: 1_000_000,
        debug: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--quick" => {
                options.sizes = vec![100_000, 1_000_000];
                options.accuracy_points = 20_000;
                options.cpu_points = 200_000;
            }
            "--big" => options.sizes = vec![1_000_000, 4_000_000, 16_000_000],
            "--sizes" => {
                options.sizes = args
                    .next()
                    .expect("--sizes a,b,c")
                    .split(',')
                    .map(|s| s.trim().parse().expect("entier"))
                    .collect();
            }
            "--adapter" => options.adapter = args.next(),
            "--debug" => options.debug = true,
            other => panic!("option inconnue : {other} (--quick --big --sizes a,b --adapter nom)"),
        }
    }
    options
}

// ---------------------------------------------------------------- CPU

struct Cpu {
    scalar: f64,
    array: f64,
    threads: f64,
    thread_count: usize,
}

fn cpu_baseline(kernel: Kernel, n: usize, input: &[(f64, f64)]) -> Cpu {
    let (from, to) = kernel.crs();
    let p = proj(from, to);
    let sample: Vec<(f64, f64)> = input.iter().copied().cycle().take(n).collect();
    let few = &sample[..sample.len().min(200_000)];
    let start = Instant::now();
    let mut sink = 0.0;
    for &q in few {
        sink += p.convert(q).map_or(0.0, |o: (f64, f64)| o.0);
    }
    std::hint::black_box(sink);
    let scalar = ns(start.elapsed().as_secs_f64(), few.len());
    let mut buffer = sample.clone();
    let mut best = f64::MAX;
    for _ in 0..3 {
        buffer.copy_from_slice(&sample);
        let start = Instant::now();
        std::hint::black_box(p.convert_array(&mut buffer).ok());
        best = best.min(start.elapsed().as_secs_f64());
    }
    let array = ns(best, sample.len());
    let thread_count = std::thread::available_parallelism().map_or(1, usize::from);
    let per = sample.len().div_ceil(thread_count);
    let mut best_mt = f64::MAX;
    // Un Proj par thread, créé avant la mesure : la création coûte des dizaines de ms.
    for _ in 0..3 {
        let locals: Vec<Proj> = (0..thread_count).map(|_| proj(from, to)).collect();
        let mut buffer = sample.clone();
        let start = Instant::now();
        std::thread::scope(|scope| {
            for (part, local) in buffer.chunks_mut(per).zip(locals) {
                scope.spawn(move || {
                    std::hint::black_box(local.convert_array(part).ok());
                });
            }
        });
        best_mt = best_mt.min(start.elapsed().as_secs_f64());
    }
    Cpu {
        scalar,
        array,
        threads: ns(best_mt, sample.len()),
        thread_count,
    }
}

// ---------------------------------------------------------------- programme

fn main() {
    let options = parse_args();
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    let mut out = String::new();
    writeln!(out, "# Spike GPU : Web Mercator et UTM en WGSL\n").expect("write");
    writeln!(
        out,
        "proj-rs {} · {} points de précision · tailles {:?}\n",
        env!("CARGO_PKG_VERSION"),
        options.accuracy_points,
        options.sizes
    )
    .expect("write");

    // Données et références, une fois par noyau.
    let max_n = options.sizes.iter().copied().max().unwrap_or(0);
    let mut data = HashMap::new();
    let mut cpu = HashMap::new();
    for kernel in KERNELS {
        eprintln!("référence CPU {}", kernel.label());
        let (input, want, geographic) = reference(kernel, options.accuracy_points);
        let cpu_stats = cpu_baseline(kernel, options.cpu_points, &input);
        // Entrées des mesures de débit : on cycle les points de précision.
        let bench: Vec<(f64, f64)> = input.iter().copied().cycle().take(max_n).collect();
        data.insert(kernel, (input, want, geographic, bench));
        cpu.insert(kernel, cpu_stats);
    }

    writeln!(out, "## CPU (proj-rs, mêmes noyaux)\n").expect("write");
    writeln!(out, "| conversion | scalaire ns/pt | convert_array ns/pt | convert_array en threads ns/pt (threads) |").expect("write");
    writeln!(out, "|---|---:|---:|---:|").expect("write");
    for kernel in KERNELS {
        let c = &cpu[&kernel];
        writeln!(
            out,
            "| {} | {:.1} | {:.1} | {:.1} ({}) |",
            kernel.label(),
            c.scalar,
            c.array,
            c.threads,
            c.thread_count
        )
        .expect("write");
    }
    writeln!(out).expect("write");

    for adapter in &adapters {
        let info = adapter.get_info();
        if let Some(filter) = &options.adapter {
            if !info.name.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }
        eprintln!("adaptateur {} ({:?})", info.name, info.backend);
        writeln!(
            out,
            "## Adaptateur : {} ({:?}, {:?})\n",
            info.name, info.backend, info.device_type
        )
        .expect("write");
        if info.device_type == wgpu::DeviceType::Cpu {
            writeln!(
                out,
                "> Rendu logiciel : la précision est valide, les temps ne disent rien d'un vrai GPU.\n"
            )
            .expect("write");
        }
        let gpu = match pollster::block_on(Gpu::new(adapter)) {
            Ok(gpu) => gpu,
            Err(e) => {
                writeln!(out, "Adaptateur inutilisable : {e}\n").expect("write");
                continue;
            }
        };
        for note in &gpu.notes {
            writeln!(out, "- {note}").expect("write");
        }
        if let Some((ok, got)) = gpu.canary_ok() {
            writeln!(
                out,
                "- Contrôle double-float (two_sum, two_prod exacts) : **{}** (obtenu {got:?})",
                if ok {
                    "OK"
                } else {
                    "ÉCHEC, le chemin df est inutilisable ici"
                }
            )
            .expect("write");
        }
        writeln!(out).expect("write");

        if options.debug {
            for kernel in KERNELS {
                let (input, want, _, _) = &data[&kernel];
                for &path in &gpu.paths {
                    let work = gpu.work(8);
                    let mut raw = Vec::new();
                    gpu.process(
                        path,
                        &gpu.pipes[&(path, kernel)],
                        &work,
                        &pack(path, &input[..6]),
                        &mut raw,
                    );
                    let got = unpack(path, &raw);
                    for i in 0..6 {
                        eprintln!(
                            "{:?} {} in {:?} gpu {:?} want {:?}",
                            kernel,
                            path.name(),
                            input[i],
                            got[i],
                            want[i]
                        );
                    }
                }
            }
        }
        writeln!(out, "### Précision contre proj-rs\n").expect("write");
        writeln!(
            out,
            "| conversion | chemin | erreur max mm | moyenne mm | > 1 mm | non finis |"
        )
        .expect("write");
        writeln!(out, "|---|---|---:|---:|---:|---:|").expect("write");
        let work = gpu.work(options.accuracy_points);
        let mut raw = Vec::new();
        for kernel in KERNELS {
            let (input, want, geographic, _) = &data[&kernel];
            for &path in &gpu.paths {
                let pipe = &gpu.pipes[&(path, kernel)];
                gpu.process(path, pipe, &work, &pack(path, input), &mut raw);
                let got = unpack(path, &raw);
                let a = accuracy(kernel, &got, want, geographic);
                writeln!(
                    out,
                    "| {} | {} | {:.4} | {:.5} | {} | {} |",
                    kernel.label(),
                    path.name(),
                    a.max_mm,
                    a.mean_mm,
                    a.over_1mm,
                    a.bad
                )
                .expect("write");
            }
        }
        writeln!(out).expect("write");

        writeln!(out, "### Débit\n").expect("write");
        writeln!(
            out,
            "Noyau seul : données déjà sur le GPU. Bout en bout : conversion f64 vers paires f32, \
             envoi, calcul, lecture, conversion inverse.\n"
        )
        .expect("write");
        writeln!(out, "| conversion | chemin | points | noyau seul ns/pt | noyau seul Mpts/s | bout en bout ns/pt | dont CPU pack/unpack ns/pt |").expect("write");
        writeln!(out, "|---|---|---:|---:|---:|---:|---:|").expect("write");
        for kernel in KERNELS {
            let (_, _, _, bench_points) = &data[&kernel];
            for &path in &gpu.paths {
                let pipe = &gpu.pipes[&(path, kernel)];
                for &n in &options.sizes {
                    let chunk = n.min(CHUNK);
                    let work = gpu.work(chunk);
                    gpu.upload(&work, &pack(path, &bench_points[..chunk]));
                    gpu.compute(path, pipe, &work, chunk, 1);
                    let first = gpu.compute(path, pipe, &work, chunk, 1);
                    let reps = if first > 0.0 {
                        (0.3 / first).clamp(1.0, 20.0) as u32
                    } else {
                        20
                    };
                    let mut samples: Vec<f64> = (0..5)
                        .map(|_| gpu.compute(path, pipe, &work, chunk, reps) / f64::from(reps))
                        .collect();
                    let kernel_only = ns(median(&mut samples), chunk);

                    let mut e2e_samples = Vec::new();
                    let mut cpu_part = Vec::new();
                    let mut raw = Vec::new();
                    for _ in 0..3 {
                        let t0 = Instant::now();
                        let packed_now = pack(path, &bench_points[..n]);
                        let pack_time = t0.elapsed().as_secs_f64();
                        gpu.process(path, pipe, &work, &packed_now, &mut raw);
                        let t1 = Instant::now();
                        let back = unpack(path, &raw);
                        let unpack_time = t1.elapsed().as_secs_f64();
                        std::hint::black_box(&back);
                        e2e_samples.push(t0.elapsed().as_secs_f64());
                        cpu_part.push(pack_time + unpack_time);
                    }
                    let e2e = ns(median(&mut e2e_samples), n);
                    let cpu_ns = ns(median(&mut cpu_part), n);
                    writeln!(
                        out,
                        "| {} | {} | {} | {:.2} | {:.1} | {:.2} | {:.2} |",
                        kernel.label(),
                        path.name(),
                        n,
                        kernel_only,
                        1e3 / kernel_only,
                        e2e,
                        cpu_ns
                    )
                    .expect("write");
                }
            }
        }
        writeln!(out).expect("write");
    }
    print!("{out}");
}
