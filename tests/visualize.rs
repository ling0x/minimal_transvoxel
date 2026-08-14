//! ASCII visualisations of what the polygonizer produces.
//!
//! These are real tests — they assert — but their point is the output. Run:
//!
//! ```sh
//! cargo test --test visualize -- --nocapture
//! ```

use minimal_transvoxel::{
    CellShape, Mesh, REGULAR_CELL, TRANSITION_CELL, Vec3, polygonize_regular_cell,
    polygonize_transition_cell,
};
use std::fmt::Write;

/// Tests run in parallel, so each one builds its whole picture and prints it in
/// a single call rather than dribbling out lines that interleave with its peers.
fn emit(report: String) {
    println!("{report}");
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

// ---------------------------------------------------------------- vector math

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

fn normalize(v: Vec3) -> Vec3 {
    let len = dot(v, v).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

// -------------------------------------------------------------- ascii drawing

/// Surface shading, dimmest to brightest. Deliberately all "solid looking"
/// characters, so nothing here can be mistaken for the `- | /` cell edges.
const SHADES: &[char] = &['+', '*', '#', '%', '@'];

/// A finer ramp for pictures with no line art in them to be confused with.
const SHADES_SMOOTH: &[char] = &['.', ':', '-', '=', '+', '*', '#', '%', '@'];

/// Marks a sample that is inside solid space, and one that is outside.
const INSIDE: char = 'X';
const OUTSIDE: char = 'o';

/// How far a step along +z slides down and to the left, as a fraction of a unit.
/// This is what makes the drawing read as a box.
const DEPTH_SLANT: f32 = 0.35;

/// A character grid with a depth buffer.
///
/// The projection is the oblique one people draw cubes with by hand — x goes
/// right, y goes up, and z comes towards the viewer as a diagonal — deliberately
/// the same view as the sample diagrams, so a picture can be read straight off
/// the diagram printed beside it.
struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<char>,
    depth: Vec<f32>,
    centre: Vec3,
    scale: f32,
    /// Set to walk around to the far side of the subject.
    from_behind: bool,
    shades: &'static [char],
}

impl Canvas {
    fn new(width: usize, height: usize, centre: Vec3, scale: f32) -> Canvas {
        Canvas {
            width,
            height,
            cells: vec![' '; width * height],
            depth: vec![f32::NEG_INFINITY; width * height],
            centre,
            scale,
            from_behind: false,
            shades: SHADES,
        }
    }

    /// Uses the finer shading ramp, for pictures drawn without a cell outline.
    fn smooth(mut self) -> Canvas {
        self.shades = SHADES_SMOOTH;
        self
    }

    /// Views the subject from the other side, by turning it half a revolution
    /// about the y axis. Used for the transition cell, whose interesting face is
    /// the fine one at `z = 0`.
    fn turned_around(mut self) -> Canvas {
        self.from_behind = true;
        self
    }

    /// Into view space: relative to the centre, and turned around if asked. Doing
    /// this before shading as well as before projecting keeps the lighting
    /// consistent with whichever side we ended up looking from.
    fn to_view(&self, p: Vec3) -> Vec3 {
        let d = sub(p, self.centre);
        if self.from_behind { [-d[0], d[1], -d[2]] } else { d }
    }

    /// To column, row and depth. Columns get twice the scale of rows because
    /// character cells are about twice as tall as they are wide, which keeps
    /// spheres round.
    fn place(&self, d: Vec3) -> (f32, f32, f32) {
        (
            self.width as f32 / 2.0 + (d[0] - DEPTH_SLANT * d[2]) * self.scale * 2.0,
            self.height as f32 / 2.0 + (DEPTH_SLANT * d[2] - d[1]) * self.scale,
            // Along the viewing ray: bigger is nearer.
            DEPTH_SLANT * d[0] + DEPTH_SLANT * d[1] + d[2],
        )
    }

    fn project(&self, p: Vec3) -> (f32, f32, f32) {
        self.place(self.to_view(p))
    }

    /// Depth-tested. `only_blank` keeps the drawing off cells that already hold
    /// something, which is how the cell outline stays behind the surface instead
    /// of cutting across it.
    fn plot(&mut self, x: isize, y: isize, depth: f32, ch: char, only_blank: bool) {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return;
        }
        let i = y as usize * self.width + x as usize;
        if (only_blank && self.cells[i] != ' ') || depth <= self.depth[i] {
            return;
        }
        self.depth[i] = depth;
        self.cells[i] = ch;
    }

    /// A 3D line, stepped along whichever screen axis is longer.
    fn line(&mut self, a: Vec3, b: Vec3, ch: char, only_blank: bool) {
        let (ax, ay, az) = self.project(a);
        let (bx, by, bz) = self.project(b);
        let steps = (bx - ax).abs().max((by - ay).abs()).ceil().max(1.0);

        for i in 0..=steps as usize {
            let t = i as f32 / steps;
            self.plot(
                (ax + (bx - ax) * t).round() as isize,
                (ay + (by - ay) * t).round() as isize,
                az + (bz - az) * t,
                ch,
                only_blank,
            );
        }
    }

    /// A depth-tested filled triangle, shaded by how squarely it faces the light.
    fn triangle(&mut self, tri: [Vec3; 3]) {
        let tri = tri.map(|p| self.to_view(p));
        let normal = cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]));
        let area = dot(normal, normal).sqrt();
        if area < 1e-9 {
            return; // degenerate, nothing to shade
        }

        // Light sits just above the viewer, so a triangle facing us is brightest.
        // Back faces — the inside of the surface, seen through a hole — come out
        // dimmest rather than invisible.
        let lit = dot(normalize(normal), normalize([0.2, 0.7, 1.0]));
        let shade = self.shades[(lit.max(0.0) * (self.shades.len() - 1) as f32).round() as usize];

        let p: Vec<(f32, f32, f32)> = tri.iter().map(|&v| self.place(v)).collect();
        let min_x = p.iter().map(|v| v.0).fold(f32::MAX, f32::min).floor() as isize;
        let max_x = p.iter().map(|v| v.0).fold(f32::MIN, f32::max).ceil() as isize;
        let min_y = p.iter().map(|v| v.1).fold(f32::MAX, f32::min).floor() as isize;
        let max_y = p.iter().map(|v| v.1).fold(f32::MIN, f32::max).ceil() as isize;

        // Twice the signed screen area, the denominator of the barycentrics.
        let denom = (p[1].0 - p[0].0) * (p[2].1 - p[0].1)
            - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
        if denom.abs() < 1e-9 {
            return; // edge-on to the camera
        }

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (px, py) = (x as f32, y as f32);
                let w1 = ((px - p[0].0) * (p[2].1 - p[0].1)
                    - (p[2].0 - p[0].0) * (py - p[0].1))
                    / denom;
                let w2 = ((p[1].0 - p[0].0) * (py - p[0].1)
                    - (px - p[0].0) * (p[1].1 - p[0].1))
                    / denom;
                let w0 = 1.0 - w1 - w2;

                // A small tolerance keeps sliver triangles from vanishing.
                if w0 < -0.03 || w1 < -0.03 || w2 < -0.03 {
                    continue;
                }
                self.plot(x, y, w0 * p[0].2 + w1 * p[1].2 + w2 * p[2].2, shade, false);
            }
        }
    }

    /// A sample marker, drawn over everything else so it stays readable even
    /// where the surface passes in front of it.
    fn marker(&mut self, p: Vec3, ch: char) {
        let (x, y, _) = self.project(p);
        self.plot(x.round() as isize, y.round() as isize, f32::INFINITY, ch, false);
    }

    /// Cropped to whatever was actually drawn, so pictures do not float around
    /// inside an oversized frame.
    fn to_text(&self) -> String {
        let rows: Vec<&[char]> = self.cells.chunks(self.width).collect();
        let used: Vec<usize> = (0..self.height)
            .filter(|&y| rows[y].iter().any(|&c| c != ' '))
            .collect();
        let (Some(&top), Some(&bottom)) = (used.first(), used.last()) else {
            return String::new();
        };
        let left = (top..=bottom)
            .filter_map(|y| rows[y].iter().position(|&c| c != ' '))
            .min()
            .unwrap_or(0);

        (top..=bottom)
            .map(|y| {
                rows[y][left..].iter().collect::<String>().trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
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

/// Renders a cell's surface inside a sketch of the cell itself: edges as
/// `- | /` after the shape's own face rings, and every sample marked.
fn render(
    shape: &CellShape,
    values: &[f32],
    mesh: &Mesh,
    size: (usize, usize),
    scale: f32,
    from_behind: bool,
) -> String {
    let mut canvas = Canvas::new(size.0, size.1, [0.5, 0.5, 0.5], scale);
    if from_behind {
        canvas = canvas.turned_around();
    }

    // Surface first, then the cell edges into whatever space is left over, so
    // the box stays behind the surface instead of slicing across it.
    for i in 0..mesh.triangles.len() {
        canvas.triangle(mesh.triangle(i));
    }

    for [a, b] in cell_edges(shape) {
        let (pa, pb) = (shape.corners[a], shape.corners[b]);
        let along = sub(pb, pa);
        // Whichever axis the edge runs along picks the character that draws it.
        let axis = (0..3).max_by(|&i, &j| along[i].abs().total_cmp(&along[j].abs()));
        canvas.line(pa, pb, ['-', '|', '/'][axis.unwrap()], true);
    }

    for (i, &value) in values.iter().enumerate() {
        canvas.marker(shape.corners[i], mark(value));
    }

    canvas.to_text()
}

/// What every character in a rendered cell means.
const KEY: &str = "key:  X sample inside solid   o sample outside   \
                   +*#%@ surface, dim to bright   -|/ cell edges";

// ------------------------------------------------------------ sample diagrams

fn mark(value: f32) -> char {
    if value < 0.0 { INSIDE } else { OUTSIDE }
}

/// The cube's 8 samples, laid out like the diagram in the crate docs — the same
/// view the renderer uses, so the two panels line up.
fn regular_diagram(s: &[f32; 8]) -> String {
    let c: Vec<char> = s.iter().map(|&v| mark(v)).collect();
    format!(
        "  samples (corner index)\n\
         \x20     {2}2--------{3}3\n\
         \x20    /|        /|\n\
         \x20   {6}6--------{7}7 |\n\
         \x20   | |       | |\n\
         \x20   | {0}0------|-{1}1\n\
         \x20   |/        |/\n\
         \x20   {4}4--------{5}5",
        c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]
    )
}

/// The transition cell's fine 3x3 face beside the coarse face it feeds.
fn transition_diagram(s: &[f32; 9]) -> String {
    let c: Vec<char> = s.iter().map(|&v| mark(v)).collect();
    format!(
        "  fine face (z=0)      coarse face (z=1)\n\
         \x20   {6}6--{7}7--{8}8            {6}--------{8}\n\
         \x20   |   |   |             |        |\n\
         \x20   {3}3--{4}4--{5}5            |        |\n\
         \x20   |   |   |             |        |\n\
         \x20   {0}0--{1}1--{2}2            {0}--------{2}",
        c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8]
    )
}

/// Side by side, so a diagram sits next to what it produced.
fn beside(left: &str, right: &str, gap: usize) -> String {
    let left: Vec<&str> = left.lines().collect();
    let right: Vec<&str> = right.lines().collect();
    let width = left.iter().map(|l| l.chars().count()).max().unwrap_or(0) + gap;

    (0..left.len().max(right.len()))
        .map(|i| {
            let l = left.get(i).copied().unwrap_or("");
            let r = right.get(i).copied().unwrap_or("");
            format!("{l:<width$}{r}").trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// -------------------------------------------------------------------- the tests

#[test]
fn regular_cases_look_right() {
    let cases: [(&str, [f32; 8]); 5] = [
        ("one corner inside", [-1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
        ("an edge inside", [-1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
        ("half inside, flat cut", [-1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0]),
        // Diagonally opposite corners: the ambiguous case, resolved by cutting
        // each corner off separately rather than joining them.
        ("two opposite corners", [-1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0]),
        ("all but one inside", [1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0]),
    ];

    let mut out = format!("\n=== regular cells ===\n{KEY}\n");
    for (name, samples) in cases {
        let mesh = polygonize_regular_cell(&samples);
        let picture = render(&REGULAR_CELL, &samples, &mesh, (36, 20), 9.0, false);
        let n = mesh.triangles.len();

        writeln!(out, "\n{name} — {n} triangle{}\n", plural(n)).unwrap();
        writeln!(out, "{}", beside(&regular_diagram(&samples), &picture, 5)).unwrap();

        assert!(!mesh.triangles.is_empty(), "{name} should produce a surface");
    }
    emit(out);
}

#[test]
fn transition_cases_look_right() {
    let cases: [(&str, [f32; 9]); 3] = [
        // Only the middle of one fine edge dips inside. The coarse face sees
        // nothing at all, so the whole patch hangs off the fine face.
        ("fine detail the coarse cell misses", [
            1.0, -1.0, 1.0,
            1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
        ]),
        // A cut both resolutions agree on: the patch spans the slab and lands on
        // the coarse face exactly where the coarse cell would put it.
        ("a cut both resolutions see", [
            -1.0, -1.0, 1.0,
            -1.0, -1.0, 1.0,
            -1.0, -1.0, 1.0,
        ]),
        ("one fine corner inside", [
            -1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
        ]),
    ];

    let mut out = format!("\n=== transition cells ===\n{KEY}\n");
    for (name, samples) in cases {
        let mesh = polygonize_transition_cell(&samples);
        // Samples 9..=12 repeat 0, 2, 6 and 8, so the picture marks them too.
        let values = [
            samples[0], samples[1], samples[2], samples[3], samples[4], samples[5],
            samples[6], samples[7], samples[8],
            samples[0], samples[2], samples[6], samples[8],
        ];
        let picture = render(&TRANSITION_CELL, &values, &mesh, (36, 20), 9.0, true);
        let n = mesh.triangles.len();

        writeln!(out, "\n{name} — {n} triangle{}\n", plural(n)).unwrap();
        writeln!(out, "{}", beside(&transition_diagram(&samples), &picture, 5)).unwrap();

        assert!(!mesh.triangles.is_empty(), "{name} should produce a surface");
    }
    emit(out);
}

#[test]
fn a_meshed_sphere_looks_like_a_sphere() {
    const SIZE: usize = 20;
    let centre = SIZE as f32 / 2.0;
    let radius = SIZE as f32 * 0.4;

    let mut canvas = Canvas::new(78, 40, [centre, centre, centre], 1.7).smooth();
    let mut triangles = 0;

    // A sphere with a smaller sphere carved out of the side facing the camera,
    // so the picture shows a concave surface and not just a lit ball.
    let bite_centre = [radius * 0.6; 3];
    let density = |p: Vec3| {
        let sphere = dot(p, p).sqrt() - radius;
        let d = sub(p, bite_centre);
        let bite = dot(d, d).sqrt() - radius * 0.6;
        sphere.max(-bite)
    };

    for z in 0..SIZE {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let samples: [f32; 8] = core::array::from_fn(|i| {
                    density([
                        (x + (i & 1)) as f32 - centre,
                        (y + (i >> 1 & 1)) as f32 - centre,
                        (z + (i >> 2 & 1)) as f32 - centre,
                    ])
                });

                let mesh = polygonize_regular_cell(&samples);
                let offset = [x as f32, y as f32, z as f32];
                for i in 0..mesh.triangles.len() {
                    canvas.triangle(mesh.triangle(i).map(|p| {
                        [p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]]
                    }));
                    triangles += 1;
                }
            }
        }
    }

    emit(format!(
        "\n=== a sphere meshed cell by cell — {triangles} triangles ===\n\n{}\n",
        canvas.to_text()
    ));

    assert!(triangles > 500, "expected a substantial mesh, got {triangles}");
}

#[test]
fn triangle_counts_across_all_256_cases() {
    let mut histogram = [0usize; 8];
    for case in 0..256u32 {
        let samples: [f32; 8] = core::array::from_fn(|i| {
            if case >> i & 1 == 1 { -1.0 } else { 1.0 }
        });
        histogram[polygonize_regular_cell(&samples).triangles.len()] += 1;
    }

    let most = histogram.iter().rposition(|&n| n > 0).unwrap();

    let mut out = String::from("\n=== triangles per cell, over all 256 regular cases ===\n\n");
    for (count, &cases) in histogram[..=most].iter().enumerate() {
        writeln!(out, "  {count} |{} {cases}", "#".repeat(cases / 2)).unwrap();
    }
    emit(out);

    assert_eq!(histogram.iter().sum::<usize>(), 256);
    assert_eq!(histogram[0], 2, "only the all-empty and all-solid cells are blank");
    assert_eq!(most, 5, "no cube case should need more than 5 triangles");
}
