//! Meshes a sphere with regular cells and writes a Wavefront OBJ to stdout,
//! so the output can be eyeballed in any 3D viewer:
//!
//! ```sh
//! cargo run --example sphere_obj > sphere.obj
//! ```
//!
//! Cells are polygonized independently and their vertices are not merged across
//! cell boundaries — coincident duplicates are fine for viewing, and sharing
//! them is an indexing concern outside the algorithm.

use minimal_transvoxel::{Vec3, polygonize_regular_cell};

/// Cells per axis.
const SIZE: usize = 16;

/// Signed distance to a sphere: negative inside, matching the crate's convention.
fn density(x: f32, y: f32, z: f32) -> f32 {
    let c = SIZE as f32 / 2.0;
    let (dx, dy, dz) = (x - c, y - c, z - c);
    (dx * dx + dy * dy + dz * dz).sqrt() - SIZE as f32 * 0.35
}

fn main() {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut triangles: Vec<[usize; 3]> = Vec::new();

    for z in 0..SIZE {
        for y in 0..SIZE {
            for x in 0..SIZE {
                // Corner i of a regular cell sits at (i & 1, i >> 1 & 1, i >> 2 & 1).
                let samples: [f32; 8] = core::array::from_fn(|i| {
                    density(
                        (x + (i & 1)) as f32,
                        (y + (i >> 1 & 1)) as f32,
                        (z + (i >> 2 & 1)) as f32,
                    )
                });

                let mesh = polygonize_regular_cell(&samples);
                let base = positions.len();

                // The cell mesh is in local [0,1]³ space; shift it into the grid.
                for p in &mesh.positions {
                    positions.push([p[0] + x as f32, p[1] + y as f32, p[2] + z as f32]);
                }
                for t in &mesh.triangles {
                    triangles.push([
                        base + t[0] as usize,
                        base + t[1] as usize,
                        base + t[2] as usize,
                    ]);
                }
            }
        }
    }

    for p in &positions {
        println!("v {} {} {}", p[0], p[1], p[2]);
    }
    for t in &triangles {
        // OBJ indices are 1-based.
        println!("f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1);
    }
    eprintln!("{} vertices, {} triangles", positions.len(), triangles.len());
}
