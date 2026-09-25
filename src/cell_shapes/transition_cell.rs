use crate::cell_shapes::CellShape;

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
        &[0, 3, 4, 1], // z = 0, high-res quadrants
        &[1, 4, 5, 2],
        &[3, 6, 7, 4],
        &[4, 7, 8, 5],
        &[9, 10, 12, 11],   // z = 1, the low-res quad
        &[0, 1, 2, 10, 9],  // y = 0
        &[8, 7, 6, 11, 12], // y = 1
        &[6, 3, 0, 9, 11],  // x = 0
        &[2, 5, 8, 12, 10], // x = 1
    ],
};
