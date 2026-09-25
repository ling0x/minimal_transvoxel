use crate::Vec3;

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
