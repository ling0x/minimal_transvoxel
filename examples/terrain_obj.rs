//! Turns a Minecraft-style heightmap — columns of stacked unit cubes — into a
//! smooth mesh, and writes both to a Wavefront OBJ so they can be compared:
//!
//! ```sh
//! cargo run --example terrain_obj > terrain.obj
//! ```
//!
//! The file holds two objects side by side: `blocky`, the cubes as they are, and
//! `smooth`, the same terrain polygonized by this crate.
//!
//! The trick is entirely in what gets sampled. A voxel column height is a step
//! function, and sampling `y - height(x, z)` with that step gives back the
//! staircase. Interpolating between neighbouring column heights first turns the
//! same data into a continuous field, and the polygonizer traces the rolling
//! surface through it. Nothing about the algorithm changes — only the samples.

use minimal_transvoxel::{Vec3, polygonize_regular_cell};
use std::fmt::Write;

/// Column heights in cubes, `HEIGHTS[z][x]` — the Minecraft input. A peak in the
/// north-west, a lower ridge running south-east, low ground between them.
const HEIGHTS: [[u8; 10]; 10] = [
    [1, 1, 2, 2, 3, 3, 2, 2, 1, 1],
    [1, 2, 3, 4, 4, 4, 3, 2, 2, 1],
    [2, 3, 5, 6, 6, 5, 4, 3, 2, 2],
    [2, 4, 6, 8, 8, 6, 4, 3, 3, 2],
    [3, 4, 6, 8, 8, 6, 5, 4, 3, 2],
    [3, 4, 5, 6, 6, 5, 5, 5, 4, 3],
    [2, 3, 4, 4, 5, 5, 6, 6, 5, 3],
    [2, 2, 3, 3, 4, 5, 6, 6, 5, 4],
    [1, 2, 2, 3, 3, 4, 5, 5, 4, 3],
    [1, 1, 2, 2, 3, 3, 4, 4, 3, 2],
];

const WIDTH: usize = HEIGHTS[0].len();
const DEPTH: usize = HEIGHTS.len();

/// Cells per cube along each axis. Raising this refines the smooth mesh without
/// changing the terrain it is tracing.
const RESOLUTION: usize = 3;

/// How far apart the two objects sit in the output file.
const APART: f32 = 3.0;

// ------------------------------------------------------------- the height field

/// Smooth ramp from 0 to 1 with zero slope at both ends, which is what stops the
/// interpolated terrain from creasing along column boundaries.
fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn height_at(x: isize, z: isize) -> f32 {
    // Clamping makes the terrain run flat off the edges of the map.
    let x = x.clamp(0, WIDTH as isize - 1) as usize;
    let z = z.clamp(0, DEPTH as isize - 1) as usize;
    HEIGHTS[z][x] as f32
}

/// The heightmap read as a continuous surface: each column's height belongs to
/// its centre, and everything between is interpolated.
fn smooth_height(x: f32, z: f32) -> f32 {
    // Column centres sit at half-integers, so shift before splitting into a cell
    // index and a fraction within it.
    let (fx, fz) = (x - 0.5, z - 0.5);
    let (x0, z0) = (fx.floor(), fz.floor());
    let (tx, tz) = (smoothstep(fx - x0), smoothstep(fz - z0));
    let (x0, z0) = (x0 as isize, z0 as isize);

    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    lerp(
        lerp(height_at(x0, z0), height_at(x0 + 1, z0), tx),
        lerp(height_at(x0, z0 + 1), height_at(x0 + 1, z0 + 1), tx),
        tz,
    )
}

/// How wide a bevel to round the chunk's edges over, in cubes. See [`density`].
const BEVEL: f32 = 0.7;

/// Intersection of two signed distances, rounded off over a width of `k`.
///
/// A plain `max` is an exact intersection, but it leaves a knife-edge crease
/// where the two surfaces meet, and nothing in the marching cubes family can
/// represent a crease that does not line up with the grid — it comes out as a
/// sawtooth at cell resolution. Rounding the join over about a cell gives the
/// polygonizer a surface it can actually trace.
fn smooth_max(a: f32, b: f32, k: f32) -> f32 {
    let h = ((k - (a - b).abs()) / k).max(0.0);
    a.max(b) + h * h * k * 0.25
}

/// Negative inside the terrain, positive outside — the crate's convention.
///
/// The heightmap surface alone would give an open sheet with nothing behind it,
/// so it is intersected with the map's footprint. That closes the result into a
/// solid chunk, matching the stack of cubes it is compared against.
fn density(p: Vec3) -> f32 {
    let surface = p[1] - smooth_height(p[0], p[2]);
    let outside_map = (-p[0])
        .max(p[0] - WIDTH as f32)
        .max(-p[1])
        .max(-p[2])
        .max(p[2] - DEPTH as f32);
    smooth_max(surface, outside_map, BEVEL)
}

// -------------------------------------------------------------------- obj output

#[derive(Default)]
struct Obj {
    out: String,
    vertices: usize,
    triangles: usize,
}

impl Obj {
    fn object(&mut self, name: &str) {
        writeln!(self.out, "o {name}").unwrap();
    }

    /// Appends geometry. Triangle indices are 0-based into `positions`; OBJ wants
    /// them 1-based and counted across the whole file, so they are rebased here.
    fn push(&mut self, positions: &[Vec3], triangles: &[[u32; 3]]) {
        for p in positions {
            writeln!(self.out, "v {} {} {}", p[0], p[1], p[2]).unwrap();
        }
        for t in triangles {
            let i = |n: u32| self.vertices + n as usize + 1;
            writeln!(self.out, "f {} {} {}", i(t[0]), i(t[1]), i(t[2])).unwrap();
        }
        self.vertices += positions.len();
        self.triangles += triangles.len();
    }

    /// A quad, given its corners in counter-clockwise order seen from outside.
    fn push_quad(&mut self, ring: [Vec3; 4]) {
        self.push(&ring, &[[0, 1, 2], [0, 2, 3]]);
    }
}

/// The height of the column at `(x, z)`, or 0 off the edge of the map.
fn column(x: isize, z: isize) -> f32 {
    if (0..WIDTH as isize).contains(&x) && (0..DEPTH as isize).contains(&z) {
        HEIGHTS[z as usize][x as usize] as f32
    } else {
        0.0
    }
}

/// The stack of cubes as a closed surface: the top of each column, its base, and
/// a wall on each side only as far as the neighbouring column leaves exposed.
///
/// Emitting whole boxes instead would bury a pair of coincident faces between
/// every adjacent pair of columns — invisible, but they z-fight in a renderer
/// and double the triangle count for nothing.
fn push_blocky(obj: &mut Obj) {
    for z in 0..DEPTH as isize {
        for x in 0..WIDTH as isize {
            let h = column(x, z);
            let (x0, x1) = (x as f32, x as f32 + 1.0);
            let (z0, z1) = (z as f32, z as f32 + 1.0);

            obj.push_quad([[x0, h, z0], [x0, h, z1], [x1, h, z1], [x1, h, z0]]);
            obj.push_quad([[x0, 0.0, z0], [x1, 0.0, z0], [x1, 0.0, z1], [x0, 0.0, z1]]);

            // Each wall runs from the neighbour's height up to this column's.
            let n = column(x, z - 1);
            if n < h {
                obj.push_quad([[x0, n, z0], [x0, h, z0], [x1, h, z0], [x1, n, z0]]);
            }
            let n = column(x, z + 1);
            if n < h {
                obj.push_quad([[x0, n, z1], [x1, n, z1], [x1, h, z1], [x0, h, z1]]);
            }
            let n = column(x - 1, z);
            if n < h {
                obj.push_quad([[x0, n, z0], [x0, n, z1], [x0, h, z1], [x0, h, z0]]);
            }
            let n = column(x + 1, z);
            if n < h {
                obj.push_quad([[x1, n, z0], [x1, h, z0], [x1, h, z1], [x1, n, z1]]);
            }
        }
    }
}

fn main() {
    let tallest = *HEIGHTS.iter().flatten().max().unwrap() as usize;
    let mut obj = Obj::default();

    // --- the cubes, exactly as the heightmap describes them ------------------
    obj.object("blocky");
    push_blocky(&mut obj);
    let blocky = obj.triangles;

    // --- the same terrain, polygonized ---------------------------------------
    obj.object("smooth");
    let step = 1.0 / RESOLUTION as f32;
    let shift = WIDTH as f32 + APART;

    // A cell of margin on every side, and one cube of headroom above the peak,
    // so every face of the chunk has empty space to close against.
    let pad = 1isize;
    let cells = |n: usize| -pad..(n * RESOLUTION) as isize + pad;

    for cz in cells(DEPTH) {
        for cy in cells(tallest + 1) {
            for cx in cells(WIDTH) {
                let cell = [cx as f32, cy as f32, cz as f32];

                // Positions are always built as `(cell index + offset) * step`,
                // never `cell index * step + offset * step`. The two are equal in
                // exact arithmetic but not in floating point, and only the first
                // makes neighbouring cells agree bit-for-bit on the vertices they
                // share — which is what lets a consumer weld them by equality.
                //
                // Corner i of a regular cell sits at (i & 1, i >> 1 & 1, i >> 2 & 1).
                let samples: [f32; 8] = core::array::from_fn(|i| {
                    density([
                        (cell[0] + (i & 1) as f32) * step,
                        (cell[1] + (i >> 1 & 1) as f32) * step,
                        (cell[2] + (i >> 2 & 1) as f32) * step,
                    ])
                });

                let mesh = polygonize_regular_cell(&samples);
                if mesh.triangles.is_empty() {
                    continue;
                }

                // Cell meshes come back in local [0,1]³, so scale to the cell
                // size before placing them.
                let positions: Vec<Vec3> = mesh
                    .positions
                    .iter()
                    .map(|p| {
                        [
                            (cell[0] + p[0]) * step + shift,
                            (cell[1] + p[1]) * step,
                            (cell[2] + p[2]) * step,
                        ]
                    })
                    .collect();
                obj.push(&positions, &mesh.triangles);
            }
        }
    }

    print!("{}", obj.out);

    // Everything below goes to stderr, so stdout stays a clean OBJ.
    eprintln!("column heights ({WIDTH}x{DEPTH}, tallest {tallest}):");
    for row in HEIGHTS {
        let cells: Vec<String> = row.iter().map(|h| h.to_string()).collect();
        eprintln!("  {}", cells.join(" "));
    }
    eprintln!();
    eprintln!("blocky: {blocky} triangles (exposed faces of {} columns)", WIDTH * DEPTH);
    eprintln!(
        "smooth: {} triangles ({RESOLUTION} cells per cube)",
        obj.triangles - blocky
    );
    eprintln!("{} vertices total", obj.vertices);
}
