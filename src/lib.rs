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

mod algorithm;
mod cell_shapes;
mod mesh;

pub use crate::{
    algorithm::polygonize::polygonize,
    cell_shapes::{CellShape, regular_cell::REGULAR_CELL, transition_cell::TRANSITION_CELL},
    mesh::Mesh,
};

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
        samples[0], samples[1], samples[2], samples[3], samples[4], samples[5], samples[6],
        samples[7], samples[8], samples[0], samples[2], samples[6], samples[8],
    ];
    polygonize(&TRANSITION_CELL, &values)
}

/// A point in the cell's local space.
pub type Vec3 = [f32; 3];
