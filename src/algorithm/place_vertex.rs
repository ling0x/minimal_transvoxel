use crate::{Mesh, cell_shapes::CellShape};

/// Returns the vertex on cell edge `a`-`b`, creating it on first request.
pub(crate) fn place_vertex(
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
