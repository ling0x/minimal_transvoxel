use crate::{
    Mesh,
    algorithm::{is_solid, place_vertex::place_vertex},
    cell_shapes::CellShape,
};

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

        // Visit each edge of the face in order around the ring
        for k in 0..face.len() {
            // Endpoints of the current edge. % wraps so the last corner
            // connects back to the first (closed polygon).
            let a = face[k];
            let b = face[(k + 1) % face.len()];
            // Same side of the isosurface on both ends → no crossing
            // on this edge; skip.
            if is_solid(values[a]) == is_solid(values[b]) {
                continue;
            }

            // places the vertex at the zero crossing along the edge, via linear
            // interpolation of the sample values.
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
