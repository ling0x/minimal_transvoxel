mod place_vertex;
pub mod polygonize;

/// A sample is inside solid space when it is negative.
#[inline]
fn is_solid(value: f32) -> bool {
    value < 0.0
}
