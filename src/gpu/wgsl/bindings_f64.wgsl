@group(0) @binding(0) var<storage, read> P: array<f64>;
@group(0) @binding(1) var<storage, read> IN: array<f64>;
@group(0) @binding(2) var<storage, read_write> OUT: array<f64>;
@group(0) @binding(3) var<uniform> CNT: vec4<u32>;
@group(0) @binding(4) var<storage, read> PROG: array<vec4<u32>>;
@group(0) @binding(5) var<storage, read_write> STATUS: array<atomic<u32>>;
