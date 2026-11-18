@group(0) @binding(0) var<storage, read> P: array<f64>;
@group(0) @binding(1) var<storage, read> IN: array<f64>;
@group(0) @binding(2) var<storage, read_write> OUT: array<f64>;
@group(0) @binding(3) var<uniform> CNT: vec4<u32>;
