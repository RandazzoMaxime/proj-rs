//! Metal: wgpu compiles WGSL to MSL with fast-math, which reassociates `two_sum` and `two_prod`
//! and zeroes the double-float error terms. So the WGSL is translated here with naga, a
//! `math_mode(safe)` pragma is put on top, and the MSL goes to wgpu as a passthrough shader.

use std::collections::BTreeMap;

use naga::back::msl;

/// Workgroup size of the `main` entry point (`@workgroup_size(64)` in the interpreter).
pub(super) const GROUP_SIZE: u32 = 64;

/// The six bindings of group 0: P, IN, OUT, CNT (uniform), PROG, STATUS.
const BINDINGS: [(wgpu::BufferBindingType, bool); 6] = [
    (wgpu::BufferBindingType::Storage { read_only: true }, true),
    (wgpu::BufferBindingType::Storage { read_only: true }, true),
    (wgpu::BufferBindingType::Storage { read_only: false }, false),
    (wgpu::BufferBindingType::Uniform, true),
    (wgpu::BufferBindingType::Storage { read_only: true }, true),
    (wgpu::BufferBindingType::Storage { read_only: false }, false),
];

/// MSL source of `wgsl`, safe math on, and the MSL names of its entry points `main` and `canary`
/// (naga renames `main`, a reserved word in MSL).
pub(super) fn msl(wgsl: &str) -> Result<(String, [String; 2]), String> {
    let module = naga::front::wgsl::parse_str(wgsl).map_err(|e| e.to_string())?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| e.to_string())?;
    // wgpu numbers the Metal buffer slots of a compute pipeline in binding order.
    let mut resources = BTreeMap::new();
    for (binding, &(_, read_only)) in (0u32..).zip(&BINDINGS) {
        resources.insert(
            naga::ResourceBinding { group: 0, binding },
            msl::BindTarget { buffer: Some(binding as u8), mutable: !read_only, ..Default::default() },
        );
    }
    let mut options = msl::Options::default();
    for entry in ["main", "canary"] {
        options.per_entry_point_map.insert(
            entry.to_owned(),
            msl::EntryPointResources { resources: resources.clone(), ..Default::default() },
        );
    }
    let (source, translation) =
        msl::write_string(&module, &info, &options, &msl::PipelineOptions::default())
            .map_err(|e| e.to_string())?;
    let mut names = [String::new(), String::new()];
    for (point, name) in module.entry_points.iter().zip(&translation.entry_point_names) {
        let slot = if point.name == "main" { 0 } else { 1 };
        names[slot] = name.clone().map_err(|e| e.to_string())?;
    }
    Ok((format!("#pragma METAL fp math_mode(safe)\n{source}"), names))
}

/// The explicit pipeline layout a passthrough shader needs.
pub(super) fn layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    let entries: Vec<_> = (0u32..)
        .zip(&BINDINGS)
        .map(|(binding, &(ty, _))| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        })
        .collect();
    let group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("proj-rs"),
        entries: &entries,
    });
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("proj-rs"),
        bind_group_layouts: &[Some(&group)],
        immediate_size: 0,
    })
}
