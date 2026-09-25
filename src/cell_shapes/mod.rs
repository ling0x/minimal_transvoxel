use crate::Vec3;

pub mod regular_cell;
pub mod transition_cell;

/// The static shape of a cell: where its samples sit and how they bound it.
pub struct CellShape {
    /// Position of each sample in local space.
    pub corners: &'static [Vec3],
    /// The cell's faces. Each is a ring of corner indices, ordered
    /// counter-clockwise as seen from *outside* the cell. Consecutive entries
    /// (wrapping at the end) are the cell's edges.
    pub faces: &'static [&'static [usize]],
}
