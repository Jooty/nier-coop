mod double_rule;
mod grid_ground;
mod hatching;
mod section_heading;
mod spaced_caps;
mod square_marks;

pub use double_rule::{DOUBLE_RULE_HEIGHT_PX, double_rule, paint_double_rule};
pub use grid_ground::paint_grid_ground;
pub use hatching::paint_hatching;
pub use section_heading::section_heading;
pub use spaced_caps::{caps_galley, caps_label, spaced_job};
pub use square_marks::{
    paint_open_square, paint_square, paint_triangle_left, paint_triangle_right,
};
