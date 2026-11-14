#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadingChoice {
    pub line_index: usize,
    pub text: String,
    pub partner_name_chars: usize,
}
