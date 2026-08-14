//! Exhaustive checks over all 256 regular and all 512 transition cases.

use minimal_transvoxel::{
    CellShape, Mesh, REGULAR_CELL, TRANSITION_CELL, Vec3, polygonize_regular_cell,
    polygonize_transition_cell,
};

/// Turns a case number into one sample per corner: bit set = solid = negative.
fn samples<const N: usize>(case: u32) -> [f32; N] {
    let mut out = [1.0; N];
    for (i, value) in out.iter_mut().enumerate() {
        if case >> i & 1 == 1 {
            *value = -1.0;
        }
    }
    out
}

/// Every unique cell edge, as a sorted pair of corner indices.
fn cell_edges(shape: &CellShape) -> Vec<[usize; 2]> {
    let mut edges: Vec<[usize; 2]> = Vec::new();
    for face in shape.faces {
        for k in 0..face.len() {
            let (a, b) = (face[k], face[(k + 1) % face.len()]);
            let edge = if a < b { [a, b] } else { [b, a] };
            if !edges.contains(&edge) {
                edges.push(edge);
            }
        }
    }
    edges
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normal(tri: [Vec3; 3]) -> Vec3 {
    cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]))
}

/// The mesh is a surface patch, so its triangles must share every edge in pairs
/// except along its boundary — and that boundary must be the closed contour
/// loops on the cell's faces, one segment per vertex.
fn assert_patch_is_well_formed(mesh: &Mesh, case: u32) {
    let mut directed: Vec<(u32, u32)> = Vec::new();
    for tri in &mesh.triangles {
        directed.push((tri[0], tri[1]));
        directed.push((tri[1], tri[2]));
        directed.push((tri[2], tri[0]));
    }

    let boundary: Vec<(u32, u32)> = directed
        .iter()
        .copied()
        .filter(|&(u, v)| !directed.contains(&(v, u)))
        .collect();

    assert_eq!(
        boundary.len(),
        mesh.positions.len(),
        "case {case}: patch boundary should visit every vertex exactly once"
    );

    for v in 0..mesh.positions.len() as u32 {
        let out = boundary.iter().filter(|&&(u, _)| u == v).count();
        let into = boundary.iter().filter(|&&(_, w)| w == v).count();
        assert_eq!(out, 1, "case {case}: vertex {v} has {out} outgoing boundary edges");
        assert_eq!(into, 1, "case {case}: vertex {v} has {into} incoming boundary edges");
    }
}

#[test]
fn uniform_cells_produce_nothing() {
    for value in [-1.0, 1.0] {
        assert!(polygonize_regular_cell(&[value; 8]).triangles.is_empty());
        assert!(polygonize_transition_cell(&[value; 9]).triangles.is_empty());
    }
}

#[test]
fn one_solid_corner_gives_one_outward_triangle() {
    // Only corner 0, at the origin, is inside.
    let mesh = polygonize_regular_cell(&[-1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);

    assert_eq!(mesh.positions.len(), 3);
    assert_eq!(mesh.triangles.len(), 1);

    // Its corner vertices sit halfway along the three edges leaving corner 0.
    let mut corners = mesh.triangle(0);
    corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(corners, [[0.0, 0.0, 0.5], [0.0, 0.5, 0.0], [0.5, 0.0, 0.0]]);

    // The normal points away from the solid corner, into empty space.
    assert!(dot(normal(mesh.triangle(0)), [1.0, 1.0, 1.0]) > 0.0);
}

#[test]
fn flat_boundary_gives_a_flat_quad() {
    // Solid below z = 0.5, empty above: corners 0..3 negative, 4..7 positive.
    let mesh = polygonize_regular_cell(&[-1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0]);

    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.triangles.len(), 2);
    assert!(mesh.positions.iter().all(|p| p[2] == 0.5));

    for i in 0..mesh.triangles.len() {
        assert!(dot(normal(mesh.triangle(i)), [0.0, 0.0, 1.0]) > 0.0);
    }
}

#[test]
fn all_256_regular_cases_are_well_formed() {
    let edges = cell_edges(&REGULAR_CELL);

    for case in 0..256u32 {
        let values: [f32; 8] = samples(case);
        let mesh = polygonize_regular_cell(&values);

        let crossings = edges
            .iter()
            .filter(|[a, b]| (values[*a] < 0.0) != (values[*b] < 0.0))
            .count();
        assert_eq!(
            mesh.positions.len(),
            crossings,
            "case {case}: one vertex per crossed cell edge"
        );

        assert_patch_is_well_formed(&mesh, case);
    }
}

#[test]
fn all_512_transition_cases_are_well_formed() {
    let edges = cell_edges(&TRANSITION_CELL);

    for case in 0..512u32 {
        let high: [f32; 9] = samples(case);
        // Samples 9..=12 repeat 0, 2, 6 and 8.
        let values = [
            high[0], high[1], high[2], high[3], high[4], high[5], high[6], high[7], high[8],
            high[0], high[2], high[6], high[8],
        ];
        let mesh = polygonize_transition_cell(&high);

        let crossings = edges
            .iter()
            .filter(|[a, b]| (values[*a] < 0.0) != (values[*b] < 0.0))
            .count();
        assert_eq!(
            mesh.positions.len(),
            crossings,
            "case {case}: one vertex per crossed cell edge"
        );

        assert_patch_is_well_formed(&mesh, case);
    }
}

#[test]
fn transition_cell_seams_exactly_onto_the_low_res_cell() {
    // The whole point of the algorithm: the coarse face of a transition cell and
    // the face of the coarse cell behind it must be cut in exactly the same
    // places, or the two meshes leave a crack.
    for case in 0..512u32 {
        let high: [f32; 9] = samples(case);
        let transition = polygonize_transition_cell(&high);

        // Match the coarse cell's z = 0 corners to the transition cell's z = 1
        // corners by position: (0,0), (1,0), (0,1), (1,1). Its far side is empty.
        let coarse = polygonize_regular_cell(&[
            high[0], high[2], high[6], high[8], 1.0, 1.0, 1.0, 1.0,
        ]);

        let mut seam: Vec<[f32; 2]> = transition
            .positions
            .iter()
            .filter(|p| p[2] == 1.0)
            .map(|p| [p[0], p[1]])
            .collect();
        let mut coarse_seam: Vec<[f32; 2]> = coarse
            .positions
            .iter()
            .filter(|p| p[2] == 0.0)
            .map(|p| [p[0], p[1]])
            .collect();

        seam.sort_by(|a, b| a.partial_cmp(b).unwrap());
        coarse_seam.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(seam, coarse_seam, "case {case}: seam vertices disagree");
    }
}

#[test]
fn a_sphere_meshed_cell_by_cell_encloses_the_right_volume() {
    // End-to-end check that neighbouring cells agree: sum the signed tetrahedron
    // volumes of every triangle (the divergence theorem). This only lands on the
    // true volume if the patches join without cracks or overlaps, and it only
    // comes out positive if every triangle is wound outward.
    const SIZE: usize = 24;
    let centre = SIZE as f32 / 2.0;
    let radius = SIZE as f32 * 0.35;
    let sphere = |x: f32, y: f32, z: f32| {
        let (dx, dy, dz) = (x - centre, y - centre, z - centre);
        (dx * dx + dy * dy + dz * dz).sqrt() - radius
    };

    let mut volume = 0.0f64;
    for z in 0..SIZE {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let values: [f32; 8] = core::array::from_fn(|i| {
                    sphere(
                        (x + (i & 1)) as f32,
                        (y + (i >> 1 & 1)) as f32,
                        (z + (i >> 2 & 1)) as f32,
                    )
                });

                let mesh = polygonize_regular_cell(&values);
                let offset = [x as f32, y as f32, z as f32];
                for i in 0..mesh.triangles.len() {
                    let t = mesh.triangle(i).map(|p| {
                        [p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]]
                    });
                    volume += dot(t[0], cross(t[1], t[2])) as f64 / 6.0;
                }
            }
        }
    }

    let exact = 4.0 / 3.0 * std::f64::consts::PI * (radius as f64).powi(3);
    let error = (volume - exact).abs() / exact;
    assert!(error < 0.01, "volume {volume:.2} vs {exact:.2} ({error:.3} off)");
}

#[test]
fn neighbouring_cells_weld_by_exact_equality() {
    // Cells are polygonized in isolation, so the only thing joining their meshes
    // is that both land a shared edge's vertex in exactly the same place. Doing
    // it bit-for-bit, not just to within a tolerance, is what lets a consumer
    // weld chunks by plain equality — so assert it that way.
    const SIZE: usize = 8;
    // A step that has no exact binary representation, to keep the arithmetic
    // honest rather than accidentally landing on round numbers.
    let step = 1.0f32 / 3.0;
    let centre = SIZE as f32 * step / 2.0;
    let sphere = |p: Vec3| {
        let d = sub(p, [centre; 3]);
        dot(d, d).sqrt() - SIZE as f32 * step * 0.35
    };

    let bits = |p: Vec3| [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
    let mut edges: Vec<([u32; 3], [u32; 3])> = Vec::new();

    for z in 0..SIZE {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let cell = [x as f32, y as f32, z as f32];
                let samples: [f32; 8] = core::array::from_fn(|i| {
                    sphere([
                        (cell[0] + (i & 1) as f32) * step,
                        (cell[1] + (i >> 1 & 1) as f32) * step,
                        (cell[2] + (i >> 2 & 1) as f32) * step,
                    ])
                });

                let mesh = polygonize_regular_cell(&samples);
                for i in 0..mesh.triangles.len() {
                    let t = mesh.triangle(i).map(|p| {
                        bits([
                            (cell[0] + p[0]) * step,
                            (cell[1] + p[1]) * step,
                            (cell[2] + p[2]) * step,
                        ])
                    });
                    edges.push((t[0], t[1]));
                    edges.push((t[1], t[2]));
                    edges.push((t[2], t[0]));
                }
            }
        }
    }

    assert!(!edges.is_empty(), "the sphere should have produced a mesh");

    let mut sorted = edges.clone();
    sorted.sort();
    for (from, to) in edges {
        assert!(
            sorted.binary_search(&(to, from)).is_ok(),
            "an edge has no oppositely-wound twin, so the cell meshes do not join"
        );
    }
}

#[test]
fn swapping_solid_and_empty_keeps_the_same_vertices() {
    // Which edges are crossed depends only on where the signs change, so the
    // vertex positions are unchanged. (The triangulation itself need not be
    // identical: ambiguous faces always separate the solid corners, so the
    // choice follows the solid, not the geometry.)
    for case in 0..256u32 {
        let values: [f32; 8] = samples(case);
        let flipped: [f32; 8] = core::array::from_fn(|i| -values[i]);

        let mut a = polygonize_regular_cell(&values).positions;
        let mut b = polygonize_regular_cell(&flipped).positions;
        a.sort_by(|x, y| x.partial_cmp(y).unwrap());
        b.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert_eq!(a, b, "case {case}");
    }
}
