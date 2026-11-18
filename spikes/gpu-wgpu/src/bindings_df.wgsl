@group(0) @binding(0) var<storage, read> P: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read> IN: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> OUT: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> CNT: vec4<u32>;
