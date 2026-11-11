mod assets;
mod install;
mod name_tag;
mod partner_view;
mod settings;

pub(crate) use assets::load_look;
pub(crate) use install::install;
pub(crate) use name_tag::draw_name_tag;
pub(crate) use partner_view::tick;
pub(crate) use settings::draw_hud_settings;

const TAG_TEXT_CELL_REFERENCE_PIXELS: f32 = 40.0;
const TAG_LIFT_REFERENCE_PIXELS: f32 = 6.0;
const ARROW_HALF_LENGTH_REFERENCE_PIXELS: f32 = 16.0;
const ARROW_HALF_WIDTH_REFERENCE_PIXELS: f32 = 11.0;
const ARROW_OUTLINE_REFERENCE_PIXELS: f32 = 2.0;
const ARROW_LABEL_GAP_REFERENCE_PIXELS: f32 = 34.0;
const TEXT_SHADOW_OFFSET_REFERENCE_PIXELS: f32 = 1.5;
const TEXT_SHADOW_ALPHA: f32 = 0.85;
const TAG_STATE_LINES_LOGGED_AT_MOST: u32 = 40;
