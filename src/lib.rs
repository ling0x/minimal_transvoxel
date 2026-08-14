//! A minimal, `std`-only polygonizer for the two cell types of the Transvoxel
//! algorithm (Eric Lengyel, <https://transvoxel.org/>).
//!
//! Two functions, no grid, no chunks, no LOD bookkeeping — just
//! `samples in, triangles out`, so the algorithm can be tested on its own:
//!
//! - [`polygonize_regular_cell`] — a cube with 8 corner samples (Marching Cubes).
//! - [`polygonize_transition_cell`] — the half-resolution seam cell with 9
//!   high-resolution samples, which is what Transvoxel adds on top of Marching Cubes.
//!
//! # Sign convention
//!
//! A sample is **inside solid space when its value is negative**, matching the
//! paper (filled dots in its figures). The isosurface is the zero level set; for
//! a different threshold, subtract it from every sample before calling.
//!
//! Triangles are wound counter-clockwise when viewed from empty space, so the
//! right-hand-rule normal points out of the solid.
//!
//! # How it works
//!
//! Both cell types are handled by one routine, [`polygonize`], because both are
//! just convex polyhedra whose faces are planar rings of samples:
//!
//! 1. On every face, walk the ring and place a vertex on each edge whose two
//!    endpoints straddle the isosurface (linear interpolation).
//! 2. Pair those vertices into directed segments, each one bracketing a run of
//!    solid corners along the ring.
//! 3. Every vertex sits on a cell edge shared by exactly two faces, so it gets
//!    exactly one incoming and one outgoing segment: the segments chain into
//!    closed loops.
//! 4. Fan-triangulate each loop.
//!
//! This *derives* the 256 regular and 512 transition cases instead of looking
//! them up in Lengyel's published tables. It is crack-free for the same reason
//! his tables are — a face's contour depends only on that face's samples, so two
//! cells sharing a face always agree on it — but on ambiguous faces (a saddle,
//! where four corners alternate sign) it commits to one fixed reading: solid
//! runs are separated rather than joined. Lengyel's tables resolve some of those
//! cases the other way, so a triangulation may differ from his there while
//! describing an equally valid, equally watertight surface.

/// A point in the cell's local space.
pub type Vec3 = [f32; 3];

/// The triangles produced for a single cell, in the cell's local space.
///
/// The unit cube `[0, 1]³` for both cell types; scale and translate to place it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    /// Vertex positions, one per cell edge crossed by the isosurface.
    pub positions: Vec<Vec3>,
    /// Triangles as indices into [`positions`](Mesh::positions).
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Convenience accessor: the three corner positions of triangle `i`.
    pub fn triangle(&self, i: usize) -> [Vec3; 3] {
        let [a, b, c] = self.triangles[i];
        [
            self.positions[a as usize],
            self.positions[b as usize],
            self.positions[c as usize],
        ]
    }
}

/// The static shape of a cell: where its samples sit and how they bound it.
pub struct CellShape {
    /// Position of each sample in local space.
    pub corners: &'static [Vec3],
    /// The cell's faces. Each is a ring of corner indices, ordered
    /// counter-clockwise as seen from *outside* the cell. Consecutive entries
    /// (wrapping at the end) are the cell's edges.
    pub faces: &'static [&'static [usize]],
}

/// A regular cell: the unit cube.
///
/// Corner `i` sits at `(i & 1, (i >> 1) & 1, (i >> 2) & 1)`, i.e.
/// ```text
///        2---------3          y   z
///       /|        /|          |  /
///      6---------7 |          | /
///      | |       | |          o----x
///      | 0-------|-1
///      |/        |/
///      4---------5
/// ```
pub const REGULAR_CELL: CellShape = CellShape {
    corners: &[
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    ],
    faces: &[
        &[0, 2, 3, 1], // z = 0
        &[4, 5, 7, 6], // z = 1
        &[0, 1, 5, 4], // y = 0
        &[3, 2, 6, 7], // y = 1
        &[2, 0, 4, 6], // x = 0
        &[1, 3, 7, 5], // x = 1
    ],
};

/// A transition cell: a slab whose `z = 0` face carries the 3x3 grid of
/// high-resolution samples and whose `z = 1` face is the single low-resolution
/// quad on the other side of the LOD boundary.
///
/// ```text
///   high-res face (z = 0)      low-res face (z = 1)
///        6---7---8                 11-------12
///        |   |   |                 |         |
///        3---4---5                 |         |
///        |   |   |                 |         |
///        0---1---2                 9--------10
/// ```
///
/// Samples 9..=12 are not independent: they repeat samples 0, 2, 6 and 8, which
/// is exactly what makes the seam close. The `z = 1` face then sees the same
/// four values as the coarse cell abutting it, so both polygonize that face
/// identically; the four side faces are pentagons that reconcile the two
/// high-resolution crossings along each edge with the single coarse one.
///
/// The slab is a full unit deep here so the output is easy to inspect. Real
/// integrations squash it against the boundary and shift the high-resolution
/// vertices inward instead; that is a placement detail, not part of the
/// polygonization.
pub const TRANSITION_CELL: CellShape = CellShape {
    corners: &[
        [0.0, 0.0, 0.0],
        [0.5, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.5, 0.0],
        [0.5, 0.5, 0.0],
        [1.0, 0.5, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    ],
    faces: &[
        &[0, 3, 4, 1],     // z = 0, high-res quadrants
        &[1, 4, 5, 2],
        &[3, 6, 7, 4],
        &[4, 7, 8, 5],
        &[9, 10, 12, 11],  // z = 1, the low-res quad
        &[0, 1, 2, 10, 9], // y = 0
        &[8, 7, 6, 11, 12], // y = 1
        &[6, 3, 0, 9, 11], // x = 0
        &[2, 5, 8, 12, 10], // x = 1
    ],
};

/// A sample is inside solid space when it is negative.
#[inline]
fn is_solid(value: f32) -> bool {
    value < 0.0
}

/// Triangulates one cube from its 8 corner samples, indexed as in [`REGULAR_CELL`].
pub fn polygonize_regular_cell(samples: &[f32; 8]) -> Mesh {
    polygonize(&REGULAR_CELL, samples)
}

/// Triangulates one transition cell from the 9 high-resolution samples of its
/// `z = 0` face, indexed as in [`TRANSITION_CELL`].
///
/// The four low-resolution samples are derived, not passed: they are copies of
/// samples 0, 2, 6 and 8.
pub fn polygonize_transition_cell(samples: &[f32; 9]) -> Mesh {
    let values = [
        samples[0], samples[1], samples[2],
        samples[3], samples[4], samples[5],
        samples[6], samples[7], samples[8],
        samples[0], samples[2], samples[6], samples[8],
    ];
    polygonize(&TRANSITION_CELL, &values)
}

/// Triangulates any [`CellShape`] from one sample per corner.
///
/// # Panics
///
/// If `values.len()` differs from `shape.corners.len()`.
pub fn polygonize(shape: &CellShape, values: &[f32]) -> Mesh {
    assert_eq!(
        values.len(),
        shape.corners.len(),
        "expected one sample per cell corner"
    );

    let mut mesh = Mesh::default();
    // Vertices already placed, keyed by the cell edge they sit on. Two faces
    // share each edge and must reuse the same vertex, or the segments below
    // would not chain up.
    let mut placed: Vec<([usize; 2], u32)> = Vec::new();
    // Directed contour segments, collected across all faces.
    let mut segments: Vec<(u32, u32)> = Vec::new();

    for face in shape.faces {
        // Walking the ring counter-clockwise from outside, a segment runs from
        // the crossing where we step into solid to the one where we step back
        // out, which orients it so the finished loop is counter-clockwise seen
        // from empty space.
        let mut entered: Option<u32> = None;
        // A ring may start part-way through a solid run, leaving its first exit
        // unmatched until the walk wraps around.
        let mut unmatched_exit: Option<u32> = None;

        for k in 0..face.len() {
            let a = face[k];
            let b = face[(k + 1) % face.len()];
            if is_solid(values[a]) == is_solid(values[b]) {
                continue;
            }

            let vertex = place_vertex(shape, values, &mut mesh, &mut placed, a, b);
            if is_solid(values[b]) {
                entered = Some(vertex);
            } else if let Some(start) = entered.take() {
                segments.push((start, vertex));
            } else {
                unmatched_exit = Some(vertex);
            }
        }

        if let (Some(start), Some(end)) = (entered, unmatched_exit) {
            segments.push((start, end));
        }
    }

    // Chain the segments head-to-tail into closed loops and fan them out.
    let mut next = vec![u32::MAX; mesh.positions.len()];
    for &(from, to) in &segments {
        next[from as usize] = to;
    }

    let mut visited = vec![false; mesh.positions.len()];
    for start in 0..mesh.positions.len() as u32 {
        if visited[start as usize] {
            continue;
        }

        let mut loop_vertices = Vec::new();
        let mut current = start;
        while !visited[current as usize] {
            visited[current as usize] = true;
            loop_vertices.push(current);
            current = next[current as usize];
        }

        for i in 1..loop_vertices.len().saturating_sub(1) {
            mesh.triangles
                .push([loop_vertices[0], loop_vertices[i], loop_vertices[i + 1]]);
        }
    }

    mesh
}

/// Returns the vertex on cell edge `a`-`b`, creating it on first request.
fn place_vertex(
    shape: &CellShape,
    values: &[f32],
    mesh: &mut Mesh,
    placed: &mut Vec<([usize; 2], u32)>,
    a: usize,
    b: usize,
) -> u32 {
    let key = if a < b { [a, b] } else { [b, a] };
    if let Some(&(_, id)) = placed.iter().find(|&&(k, _)| k == key) {
        return id;
    }

    // Where the straight line between the two samples reaches zero. The signs
    // differ, so the denominator cannot vanish.
    //
    // Interpolating from the lower-numbered corner rather than from whichever
    // one the caller happened to name first matters in floating point: `t` and
    // `1 - t` from opposite ends land an ulp or two apart. Corner indices are
    // ordered along each axis, so two neighbouring cells walk a shared edge from
    // the same end and agree on the vertex bit-for-bit — which is what lets a
    // consumer weld their meshes by plain equality.
    let [a, b] = key;
    let (va, vb) = (values[a], values[b]);
    let t = va / (va - vb);
    let (pa, pb) = (shape.corners[a], shape.corners[b]);
    let position = [
        pa[0] + t * (pb[0] - pa[0]),
        pa[1] + t * (pb[1] - pa[1]),
        pa[2] + t * (pb[2] - pa[2]),
    ];

    let id = mesh.positions.len() as u32;
    mesh.positions.push(position);
    placed.push((key, id));
    id
}
