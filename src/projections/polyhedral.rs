//! Snyder equal-area polyhedra: `tsea`, `dsea`, `isea`, and `ivea`.
//!
//! The net is unfolded from the default orientation, then the user's
//! `orient_lat` / `orient_lon` / `azi` rotate the sphere. `lat_0` / `lon_0`
//! name a projected origin; PROJ clears `lam0` so they are not a longitude
//! shift. On an ellipsoid the kernel scales by `sqrt(qp / 2)` and the classic
//! wrapper still multiplies by the original `a`.

#![allow(
    clippy::pedantic,
    reason = "the unfold, Conway subdivision, and Snyder map follow polyhedral.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{authalic_forward, authalic_inverse, authalic_q};
use std::f64::consts::{FRAC_PI_2, PI};

const ONE_TOL: f64 = 1.000_000_000_000_01;
const ATOL: f64 = 1e-50;
const BARY_EPS: f64 = 1e-10;

const TSEA: [i32; 4] = [2, 3, 0, 3];
const TSEA_TRIANGLE: [i32; 4] = [3, 3, 0, 3];
const DSEA: [i32; 12] = [4, 7, 8, 0, 9, 10, 3, 4, 4, 5, 6, 9];
const DSEA_A5: [i32; 12] = [4, 6, 8, 9, 6, 0, 2, 12, 10, 5, 7, 11];
const DSEA_CRESCENT: [i32; 12] = [4, 3, 4, 0, 4, 5, 8, 9, 4, 9, 10, 9];
const DSEA_FLOWER: [i32; 12] = [4, 1, 1, 0, 1, 1, 12, 12, 4, 12, 12, 9];
const ISEA: [i32; 20] = [
    6, 7, 8, 9, 10, 11, 12, 0, 13, 14, 7, 8, 8, 9, 10, 11, 12, 13, 14, 15,
];

struct Mesh {
    verts: Vec<[f64; 3]>,
    faces: Vec<Vec<usize>>,
}

struct Tri {
    sph: [[f64; 3]; 3],
    face: [[f64; 2]; 3],
}

struct Spec {
    mesh: Mesh,
    parents: &'static [i32],
    dual: bool,
    orient_lat: f64,
    orient_lon: f64,
    azi: f64,
    bbox_origin: bool,
}

struct Poly {
    tris: Vec<Tri>,
    orient: [[f64; 3]; 3],
    orient_inv: [[f64; 3]; 3],
    x_off: f64,
    y_off: f64,
    rq: f64,
    ellipsoidal: bool,
    n: f64,
    e: f64,
    es: f64,
    one_es: f64,
    qp: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "tsea" | "dsea" => Some(&["net", "dual", "orient_lat", "orient_lon", "azi"]),
        "isea" => Some(&["net", "dual", "orient", "orient_lat", "orient_lon", "azi"]),
        "ivea" => Some(&["orient", "orient_lat", "orient_lon", "azi"]),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let spec = match name {
        "tsea" => tsea_spec(params)?,
        "dsea" => {
            if params.flag("dual") {
                icosa_spec(params, frame, true, false)?
            } else {
                dodeca_spec(params, false)?
            }
        }
        "isea" => {
            if params.flag("dual") {
                dodeca_spec(params, true)?
            } else {
                icosa_spec(params, frame, false, true)?
            }
        }
        "ivea" => icosa_spec(params, frame, true, true)?,
        _ => return Ok(None),
    };
    let kernel = assemble(params, frame, &spec)?;
    let mut placed = frame.clone();
    placed.lam0 = 0.0;
    placed.phi0 = 0.0;
    Ok(Some(classic_step(&placed, kernel)))
}

fn tsea_spec(params: &Params) -> Result<Spec, CreateError> {
    let parents = match params.text("net") {
        None => &TSEA,
        Some("tsea") => &TSEA,
        Some("triangle") => &TSEA_TRIANGLE,
        Some(_) => return Err(CreateError::IllegalValue("net")),
    };
    if params.flag("net") && params.text("net").is_none() {
        return Err(CreateError::IllegalValue("net"));
    }
    Ok(Spec {
        mesh: tetrahedron(),
        parents,
        dual: params.flag("dual"),
        orient_lat: 90.0,
        orient_lon: 0.0,
        azi: 0.0,
        bbox_origin: false,
    })
}

fn dodeca_spec(params: &Params, dual: bool) -> Result<Spec, CreateError> {
    let mut orient_lon = -36.0;
    let parents = match params.text("net") {
        None => &DSEA,
        Some("dsea") => &DSEA,
        Some("a5") => {
            orient_lon = -36.0 - 93.0;
            &DSEA_A5
        }
        Some("crescent") => &DSEA_CRESCENT,
        Some("flower") => &DSEA_FLOWER,
        Some(_) => return Err(CreateError::IllegalValue("net")),
    };
    if params.flag("net") && params.text("net").is_none() {
        return Err(CreateError::IllegalValue("net"));
    }
    Ok(Spec {
        mesh: dodecahedron(),
        parents,
        dual,
        orient_lat: dodeca_orient_lat(),
        orient_lon,
        azi: 240.0,
        bbox_origin: false,
    })
}

fn icosa_spec(
    params: &Params,
    frame: &Frame,
    dual: bool,
    shorthand: bool,
) -> Result<Spec, CreateError> {
    let mut spec = Spec {
        mesh: icosahedron(),
        parents: &ISEA,
        dual,
        orient_lat: 58.282_525_588_539,
        orient_lon: if frame.earth.es() == 0.0 {
            11.25
        } else {
            11.20
        },
        azi: 0.0,
        bbox_origin: true,
    };
    if shorthand {
        match params.text("orient") {
            None => {
                if params.flag("orient") {
                    return Err(CreateError::IllegalValue("orient"));
                }
            }
            Some("isea") => {}
            Some("pole") => {
                spec.orient_lat = 90.0;
                spec.orient_lon = 0.0;
                spec.azi = 0.0;
            }
            Some(_) => return Err(CreateError::IllegalValue("orient")),
        }
    }
    Ok(spec)
}

fn assemble(params: &Params, frame: &Frame, spec: &Spec) -> Result<Poly, CreateError> {
    let earth = frame.earth;
    let es = earth.es();
    let ellipsoidal = es != 0.0;
    let e = earth.e();
    let one_es = earth.one_es();
    let n = earth.n();
    let qp = if ellipsoidal {
        authalic_q(1.0, e, one_es)
    } else {
        0.0
    };
    let up = orient_up(spec.orient_lat, spec.azi);
    let net = unfold(&spec.mesh, spec.parents, up)?;
    let sph = conway(&spec.mesh.verts, &spec.mesh.faces, true, spec.dual);
    let face = conway(&net.verts, &net.faces, false, spec.dual);
    let tris = sph
        .into_iter()
        .zip(face)
        .map(|(sph_tri, face_tri)| Tri {
            sph: sph_tri,
            face: [
                [face_tri[0][0], face_tri[0][1]],
                [face_tri[1][0], face_tri[1][1]],
                [face_tri[2][0], face_tri[2][1]],
            ],
        })
        .collect::<Vec<_>>();
    let mut orient_lat = spec.orient_lat;
    let mut orient_lon = spec.orient_lon;
    let mut azi = spec.azi;
    if let Some(deg) = params.number("orient_lat")? {
        orient_lat = if ellipsoidal {
            let phi = deg.to_radians();
            authalic_forward(phi, phi.sin(), n, e, one_es, qp).to_degrees()
        } else {
            deg
        };
    }
    if let Some(deg) = params.number("orient_lon")? {
        orient_lon = deg;
    }
    if let Some(deg) = params.number("azi")? {
        azi = deg;
    }
    let (orient, orient_inv) = orient_matrix(orient_lat, orient_lon, azi);
    let mut poly = Poly {
        tris,
        orient,
        orient_inv,
        x_off: 0.0,
        y_off: 0.0,
        rq: if ellipsoidal { (0.5 * qp).sqrt() } else { 1.0 },
        ellipsoidal,
        n,
        e,
        es,
        one_es,
        qp,
    };
    let [x_off, y_off] = if params.flag("lat_0") || params.flag("lon_0") {
        let lat = if params.flag("lat_0") {
            frame.phi0
        } else {
            0.0
        };
        let lon = if params.flag("lon_0") {
            frame.lam0
        } else {
            0.0
        };
        let point = poly
            .project_net(lon, lat)
            .map_err(|_| CreateError::IllegalValue("lat_0"))?;
        point
    } else {
        let root = spec
            .parents
            .iter()
            .position(|&parent| parent == 0)
            .unwrap_or(0);
        let nfv = spec.mesh.faces.first().map_or(0, Vec::len);
        face_reference(&poly.tris, root, nfv, spec.bbox_origin)
    };
    poly.x_off = x_off;
    poly.y_off = y_off;
    Ok(poly)
}

impl Poly {
    fn project_net(&self, lam: f64, phi: f64) -> Result<[f64; 2], TransformError> {
        let vector = self.to_cart(lam, phi);
        let index = find_triangle(&self.tris, vector).ok_or(TransformError::OutsideDomain)?;
        let tri = &self.tris[index];
        Ok(snyder_fwd(vector, tri.sph, tri.face))
    }

    fn to_cart(&self, lam: f64, phi: f64) -> [f64; 3] {
        let auth = if self.ellipsoidal {
            authalic_forward(phi, phi.sin(), self.n, self.e, self.one_es, self.qp)
        } else {
            phi
        };
        let geographic = [auth.cos() * lam.cos(), auth.cos() * lam.sin(), auth.sin()];
        mat_vec(self.orient, geographic)
    }

    fn to_lonlat(&self, vector: [f64; 3]) -> Result<(f64, f64), TransformError> {
        let geographic = mat_vec(self.orient_inv, vector);
        let auth = aasin(geographic[2])?;
        let phi = if self.ellipsoidal {
            authalic_inverse(auth, self.n, self.e, self.es, self.one_es, self.qp)
        } else {
            auth
        };
        Ok((aatan2(geographic[1], geographic[0]), phi))
    }
}

impl Kernel for Poly {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let point = self.project_net(lam, phi)?;
        Ok((
            (point[0] - self.x_off) * self.rq,
            (point[1] - self.y_off) * self.rq,
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let point = [x / self.rq + self.x_off, y / self.rq + self.y_off];
        for tri in &self.tris {
            let bary = face_to_bary(point, tri.face);
            if bary[0] >= -BARY_EPS && bary[1] >= -BARY_EPS && bary[2] >= -BARY_EPS {
                let vector = snyder_inv(point, tri.face, tri.sph);
                return self.to_lonlat(vector);
            }
        }
        Err(TransformError::OutsideDomain)
    }
}

fn face_reference(tris: &[Tri], face: usize, nfv: usize, bbox: bool) -> [f64; 2] {
    let fan = nfv.saturating_mul(2);
    let start = fan.saturating_mul(face);
    let end = start.saturating_add(fan).min(tris.len());
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut count = 0.0;
    for tri in &tris[start..end] {
        for corner in tri.face {
            sum_x += corner[0];
            sum_y += corner[1];
            min_x = min_x.min(corner[0]);
            max_x = max_x.max(corner[0]);
            min_y = min_y.min(corner[1]);
            max_y = max_y.max(corner[1]);
            count += 1.0;
        }
    }
    if bbox {
        [0.5 * (min_x + max_x), 0.5 * (min_y + max_y)]
    } else if count == 0.0 {
        [0.0, 0.0]
    } else {
        [sum_x / count, sum_y / count]
    }
}

fn orient_up(lat_deg: f64, azi_deg: f64) -> [f64; 3] {
    let lat = lat_deg.to_radians();
    let azi = azi_deg.to_radians();
    [-azi.cos() * lat.cos(), -azi.sin() * lat.cos(), lat.sin()]
}

fn orient_matrix(lat_deg: f64, lon_deg: f64, azi_deg: f64) -> ([[f64; 3]; 3], [[f64; 3]; 3]) {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let azi = azi_deg.to_radians();
    let alpha = FRAC_PI_2 - lat;
    let (ca, sa) = (alpha.cos(), alpha.sin());
    let (cl, sl) = (lon.cos(), lon.sin());
    let (cz, sz) = (azi.cos(), azi.sin());
    let r00 = ca * cl;
    let r01 = ca * sl;
    let r02 = -sa;
    let r10 = -sl;
    let r11 = cl;
    let r20 = sa * cl;
    let r21 = sa * sl;
    let r22 = ca;
    let orient = [
        [cz * r00 - sz * r10, cz * r01 - sz * r11, cz * r02],
        [sz * r00 + cz * r10, sz * r01 + cz * r11, sz * r02],
        [r20, r21, r22],
    ];
    let orient_inv = [
        [orient[0][0], orient[1][0], orient[2][0]],
        [orient[0][1], orient[1][1], orient[2][1]],
        [orient[0][2], orient[1][2], orient[2][2]],
    ];
    (orient, orient_inv)
}

fn mat_vec(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn find_triangle(tris: &[Tri], vector: [f64; 3]) -> Option<usize> {
    for (index, tri) in tris.iter().enumerate() {
        if dot(vector, tri.sph[0]) <= 0.0 {
            continue;
        }
        let t1 = triple(vector, tri.sph[0], tri.sph[1]);
        let t2 = triple(vector, tri.sph[1], tri.sph[2]);
        let t3 = triple(vector, tri.sph[2], tri.sph[0]);
        let positive = t1 >= 0.0 && t2 >= 0.0 && t3 >= 0.0;
        let negative = t1 <= 0.0 && t2 <= 0.0 && t3 <= 0.0;
        if positive || negative {
            return Some(index);
        }
    }
    None
}

fn snyder_fwd(vector: [f64; 3], sph: [[f64; 3]; 3], face: [[f64; 2]; 3]) -> [f64; 2] {
    let (a, b, c) = (sph[0], sph[1], sph[2]);
    let zed = normalize(sub(vector, a));
    let mut pole = normalize(quadruple(a, zed, b, c));
    if dot(pole, b) + dot(pole, c) < 0.0 {
        pole = [-pole[0], -pole[1], -pole[2]];
    }
    let height = vector_difference(a, vector) / vector_difference(a, pole);
    let area_abc = sph_area(a, b, c);
    let scaled = height / area_abc;
    let bary = [
        1.0 - height,
        scaled * sph_area(a, pole, c),
        scaled * sph_area(a, b, pole),
    ];
    bary_to_face(bary, face)
}

fn snyder_inv(point: [f64; 2], face: [[f64; 2]; 3], sph: [[f64; 3]; 3]) -> [f64; 3] {
    let (a, b, c) = (sph[0], sph[1], sph[2]);
    let bary = face_to_bary(point, face);
    const THRESHOLD: f64 = 1.0 - 1e-14;
    if bary[0] > THRESHOLD {
        return a;
    }
    if bary[1] > THRESHOLD {
        return b;
    }
    if bary[2] > THRESHOLD {
        return c;
    }
    let cross_bc = cross(b, c);
    let area_abc = sph_area(a, b, c);
    let height = 1.0 - bary[0];
    let ratio = bary[2] / height;
    let alpha = ratio * area_abc;
    let sine = alpha.sin();
    let half = (alpha / 2.0).sin();
    let chord = 2.0 * half * half;
    let c01 = dot(a, b);
    let c12 = dot(b, c);
    let c20 = dot(c, a);
    let s12 = length(cross_bc);
    let triple_product = dot(a, cross_bc);
    let f_term = sine * triple_product + chord * (c01 * c12 - c20);
    let g_term = chord * s12 * (1.0 + c01);
    let along = (2.0 / c12.clamp(-1.0, 1.0).acos()) * g_term.atan2(f_term);
    let edge = slerp(b, c, along);
    let span = vector_difference(a, edge);
    let travel = (height * span).asin() / span.asin();
    slerp(a, edge, travel)
}

fn face_to_bary(point: [f64; 2], tri: [[f64; 2]; 3]) -> [f64; 3] {
    let d31 = [tri[0][0] - tri[2][0], tri[0][1] - tri[2][1]];
    let d23 = [tri[2][0] - tri[1][0], tri[2][1] - tri[1][1]];
    let d3p = [point[0] - tri[2][0], point[1] - tri[2][1]];
    let det = d23[0] * d31[1] - d23[1] * d31[0];
    let b0 = (d23[0] * d3p[1] - d23[1] * d3p[0]) / det;
    let b1 = (d31[0] * d3p[1] - d31[1] * d3p[0]) / det;
    [b0, b1, 1.0 - (b0 + b1)]
}

fn bary_to_face(bary: [f64; 3], tri: [[f64; 2]; 3]) -> [f64; 2] {
    [
        bary[0] * tri[0][0] + bary[1] * tri[1][0] + bary[2] * tri[2][0],
        bary[0] * tri[0][1] + bary[1] * tri[1][1] + bary[2] * tri[2][1],
    ]
}

fn unfold(poly: &Mesh, parents: &[i32], up: [f64; 3]) -> Result<Mesh, CreateError> {
    let face_count = poly.faces.len();
    let nfv = poly.faces.first().map_or(0, Vec::len);
    let Some(root) = parents.iter().position(|&parent| parent == 0) else {
        return Err(CreateError::IllegalValue("net"));
    };
    let origin_poly = centroid(&poly.verts, &poly.faces[root]);
    let normal = normalize(origin_poly);
    let step = sub(up, scale(normal, dot(up, normal)));
    let offset_poly = if length(step) < 1e-12 {
        poly.verts[poly.faces[root][0]]
    } else {
        add(origin_poly, step)
    };
    let root_matrix = rigid(
        normal,
        origin_poly,
        [0.0, 0.0, 0.0],
        offset_poly,
        [0.0, 1.0, 0.0],
    );
    let mut verts = Vec::new();
    let mut faces = vec![vec![0usize; nfv]; face_count];
    for slot in 0..nfv {
        let vertex = apply(root_matrix, poly.verts[poly.faces[root][slot]]);
        faces[root][slot] = verts.len();
        verts.push(vertex);
    }
    let mut queue = Vec::new();
    let root_id = i32::try_from(root + 1).map_err(|_| CreateError::IllegalValue("net"))?;
    for (index, parent) in parents.iter().enumerate() {
        if *parent == root_id {
            queue.push(index);
        }
    }
    let mut head = 0usize;
    while head < queue.len() {
        let child = queue[head];
        head += 1;
        let parent =
            usize::try_from(parents[child] - 1).map_err(|_| CreateError::IllegalValue("net"))?;
        let mut shared = Vec::new();
        for (child_slot, child_vertex) in poly.faces[child].iter().enumerate() {
            for (parent_slot, parent_vertex) in poly.faces[parent].iter().enumerate() {
                if child_vertex == parent_vertex {
                    shared.push((child_slot, parent_slot));
                }
            }
        }
        if shared.len() < 2 {
            return Err(CreateError::IllegalValue("net"));
        }
        let (child_a, parent_a) = shared[0];
        let (child_b, parent_b) = shared[1];
        let net_a = faces[parent][parent_a];
        let net_b = faces[parent][parent_b];
        let child_normal = normalize(centroid(&poly.verts, &poly.faces[child]));
        let matrix = rigid(
            child_normal,
            poly.verts[poly.faces[child][child_a]],
            verts[net_a],
            poly.verts[poly.faces[child][child_b]],
            verts[net_b],
        );
        for slot in 0..nfv {
            if slot == child_a {
                faces[child][slot] = net_a;
            } else if slot == child_b {
                faces[child][slot] = net_b;
            } else {
                let vertex = apply(matrix, poly.verts[poly.faces[child][slot]]);
                faces[child][slot] = verts.len();
                verts.push(vertex);
            }
        }
        let child_id = i32::try_from(child + 1).map_err(|_| CreateError::IllegalValue("net"))?;
        for (index, parent_id) in parents.iter().enumerate() {
            if *parent_id == child_id {
                queue.push(index);
            }
        }
    }
    let mut total = 0.0;
    for face in &faces {
        total += polygon_area(&verts, face);
    }
    let factor = (4.0 * PI / total).sqrt();
    let center = centroid(&verts, &faces[root]);
    for vertex in &mut verts {
        *vertex = scale(sub(*vertex, center), factor);
    }
    Ok(Mesh { verts, faces })
}

fn conway(
    verts: &[[f64; 3]],
    faces: &[Vec<usize>],
    sphere: bool,
    dual: bool,
) -> Vec<[[f64; 3]; 3]> {
    let mut out = Vec::new();
    for face in faces {
        let nfv = face.len();
        let fan_size = nfv.saturating_mul(2);
        let mut fan = vec![[0.0; 3]; fan_size];
        let mut center = [0.0, 0.0, 0.0];
        for (slot, vertex_index) in face.iter().enumerate() {
            let start = verts[*vertex_index];
            let end = verts[face[(slot + 1) % nfv]];
            fan[slot * 2] = start;
            fan[slot * 2 + 1] = lerp(start, end, 0.5);
            center = add(center, start);
        }
        if sphere {
            for point in &mut fan {
                *point = normalize(*point);
            }
            center = normalize(center);
        } else {
            center = scale(center, 1.0 / as_f64(nfv));
        }
        for slot in 0..fan_size {
            let even = slot % 2 == 0;
            let next = (slot + 1) % fan_size;
            let vertex = if even { fan[slot] } else { fan[next] };
            let midpoint = if even { fan[next] } else { fan[slot] };
            let tri = if dual {
                [vertex, midpoint, center]
            } else {
                [center, midpoint, vertex]
            };
            out.push(tri);
        }
    }
    out
}

fn rigid(
    normal: [f64; 3],
    poly_a: [f64; 3],
    net_a: [f64; 3],
    poly_b: [f64; 3],
    net_b: [f64; 3],
) -> [[f64; 4]; 4] {
    let axis_u = normalize(sub(poly_b, poly_a));
    let axis_v = cross(normal, axis_u);
    let edge = normalize(sub(net_b, net_a));
    let row0 = sub(scale(axis_u, edge[0]), scale(axis_v, edge[1]));
    let row1 = add(scale(axis_u, edge[1]), scale(axis_v, edge[0]));
    let shift0 = net_a[0] - dot(row0, poly_a);
    let shift1 = net_a[1] - dot(row1, poly_a);
    [
        [row0[0], row0[1], row0[2], shift0],
        [row1[0], row1[1], row1[2], shift1],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn apply(matrix: [[f64; 4]; 4], vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0]
            + matrix[0][1] * vector[1]
            + matrix[0][2] * vector[2]
            + matrix[0][3],
        matrix[1][0] * vector[0]
            + matrix[1][1] * vector[1]
            + matrix[1][2] * vector[2]
            + matrix[1][3],
        matrix[2][0] * vector[0]
            + matrix[2][1] * vector[1]
            + matrix[2][2] * vector[2]
            + matrix[2][3],
    ]
}

fn polygon_area(verts: &[[f64; 3]], face: &[usize]) -> f64 {
    let mut area = 0.0;
    let nfv = face.len();
    for slot in 0..nfv {
        let start = verts[face[slot]];
        let end = verts[face[(slot + 1) % nfv]];
        area += start[0] * end[1] - start[1] * end[0];
    }
    area.abs() * 0.5
}

fn centroid(verts: &[[f64; 3]], face: &[usize]) -> [f64; 3] {
    let mut sum = [0.0, 0.0, 0.0];
    for index in face {
        sum = add(sum, verts[*index]);
    }
    scale(sum, 1.0 / as_f64(face.len()))
}

fn quadruple(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> [f64; 3] {
    let crossed = cross(c, d);
    sub(scale(b, dot(a, crossed)), scale(a, dot(b, crossed)))
}

fn vector_difference(a: [f64; 3], b: [f64; 3]) -> f64 {
    let apart = length(sub(a, b));
    let together = length(add(a, b));
    apart / apart.hypot(together)
}

fn sph_area(v1: [f64; 3], v2: [f64; 3], v3: [f64; 3]) -> f64 {
    let numerator = triple(v1, v2, v3);
    let denominator = 1.0 + dot(v1, v2) + dot(v2, v3) + dot(v3, v1);
    2.0 * numerator.atan2(denominator)
}

fn slerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    let gamma = angle(a, b);
    if gamma == 0.0 {
        return lerp(a, b, t);
    }
    let sine = gamma.sin();
    let weight_a = ((1.0 - t) * gamma).sin() / sine;
    let weight_b = (t * gamma).sin() / sine;
    add(scale(a, weight_a), scale(b, weight_b))
}

fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    let scaled_a = scale(a, length(b));
    let scaled_b = scale(b, length(a));
    2.0 * length(sub(scaled_a, scaled_b)).atan2(length(add(scaled_a, scaled_b)))
}

fn triple(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    dot(a, cross(b, c))
}

fn aasin(value: f64) -> Result<f64, TransformError> {
    let magnitude = value.abs();
    if magnitude >= 1.0 {
        if magnitude > ONE_TOL {
            return Err(TransformError::OutsideDomain);
        }
        return Ok(if value < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 });
    }
    Ok(value.asin())
}

fn aatan2(north: f64, east: f64) -> f64 {
    if north.abs() < ATOL && east.abs() < ATOL {
        0.0
    } else {
        north.atan2(east)
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(vector: [f64; 3]) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
}

fn normalize(vector: [f64; 3]) -> [f64; 3] {
    let norm = length(vector);
    if norm == 0.0 {
        vector
    } else {
        scale(vector, 1.0 / norm)
    }
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn lerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

fn as_f64(value: usize) -> f64 {
    u16::try_from(value).map_or(0.0, f64::from)
}

fn tetrahedron() -> Mesh {
    const A: f64 = 0.942_809_041_582_063_5;
    const B: f64 = 0.816_496_580_927_726_0;
    mesh(
        [
            [0.0, 0.0, 1.0],
            [0.5 * A, B, -1.0 / 3.0],
            [0.5 * A, -B, -1.0 / 3.0],
            [-A, 0.0, -1.0 / 3.0],
        ],
        &[[0, 3, 2], [0, 2, 1], [1, 2, 3], [0, 1, 3]],
    )
}

fn icosahedron() -> Mesh {
    const A: f64 = 0.894_427_190_999_915_878_564;
    const B: f64 = 0.447_213_595_499_957_939_282;
    const C72: f64 = 0.309_016_994_374_947_424_102;
    const S72: f64 = 0.951_056_516_295_153_572_116;
    const C36: f64 = 0.809_016_994_374_947_424_102;
    const S36: f64 = 0.587_785_252_292_473_129_169;
    mesh(
        [
            [0.0, 0.0, 1.0],
            [-A, 0.0, B],
            [-A * C72, -A * S72, B],
            [A * C36, -A * S36, B],
            [A * C36, A * S36, B],
            [-A * C72, A * S72, B],
            [-A * C36, -A * S36, -B],
            [A * C72, -A * S72, -B],
            [A, 0.0, -B],
            [A * C72, A * S72, -B],
            [-A * C36, A * S36, -B],
            [0.0, 0.0, -1.0],
        ],
        &[
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 4],
            [0, 4, 5],
            [0, 5, 1],
            [1, 6, 2],
            [2, 7, 3],
            [3, 8, 4],
            [4, 9, 5],
            [5, 10, 1],
            [6, 7, 2],
            [7, 8, 3],
            [8, 9, 4],
            [9, 10, 5],
            [10, 6, 1],
            [11, 7, 6],
            [11, 8, 7],
            [11, 9, 8],
            [11, 10, 9],
            [11, 6, 10],
        ],
    )
}

fn dodeca_orient_lat() -> f64 {
    const A: f64 = 0.809_016_994_374_947_424_102_293_417_183;
    ((1.0 + 2.0 * A) / 2.0).atan().to_degrees()
}

fn dodecahedron() -> Mesh {
    const A: f64 = 0.809_016_994_374_947_424_102_293_417_183;
    const B: f64 = 0.577_350_269_189_625_764_509_148_780_502;
    let z_ring = (4.0 * A - 1.0) / 3.0;
    mesh(
        [
            [0.0, 0.0, 1.0],
            [2.0 / 3.0, 0.0, z_ring],
            [z_ring, B, 1.0 / 3.0],
            [(2.0 - 2.0 * A) / 3.0, 2.0 * A * B, 1.0 / 3.0],
            [-1.0 / 3.0, B, z_ring],
            [-1.0 / 3.0, -B, z_ring],
            [z_ring, -B, 1.0 / 3.0],
            [(1.0 + 2.0 * A) / 3.0, (2.0 * A - 1.0) * B, -1.0 / 3.0],
            [(2.0 * A - 2.0) / 3.0, 2.0 * A * B, -1.0 / 3.0],
            [-(1.0 + 2.0 * A) / 3.0, (2.0 * A - 1.0) * B, 1.0 / 3.0],
            [(2.0 - 2.0 * A) / 3.0, -2.0 * A * B, 1.0 / 3.0],
            [(1.0 + 2.0 * A) / 3.0, -(2.0 * A - 1.0) * B, -1.0 / 3.0],
            [1.0 / 3.0, B, -z_ring],
            [-z_ring, B, -1.0 / 3.0],
            [-(1.0 + 2.0 * A) / 3.0, -(2.0 * A - 1.0) * B, 1.0 / 3.0],
            [(2.0 * A - 2.0) / 3.0, -2.0 * A * B, -1.0 / 3.0],
            [1.0 / 3.0, -B, -z_ring],
            [0.0, 0.0, -1.0],
            [-2.0 / 3.0, 0.0, -z_ring],
            [-z_ring, -B, -1.0 / 3.0],
        ],
        &[
            [0, 1, 2, 3, 4],
            [3, 8, 13, 9, 4],
            [0, 4, 9, 14, 5],
            [0, 5, 10, 6, 1],
            [1, 6, 11, 7, 2],
            [2, 7, 12, 8, 3],
            [9, 13, 18, 19, 14],
            [5, 14, 19, 15, 10],
            [6, 10, 15, 16, 11],
            [7, 11, 16, 17, 12],
            [8, 12, 17, 18, 13],
            [15, 19, 18, 17, 16],
        ],
    )
}

fn mesh<const NV: usize, const NF: usize, const NFV: usize>(
    verts: [[f64; 3]; NV],
    faces: &[[usize; NFV]; NF],
) -> Mesh {
    Mesh {
        verts: verts.to_vec(),
        faces: faces.iter().map(|face| face.to_vec()).collect(),
    }
}
