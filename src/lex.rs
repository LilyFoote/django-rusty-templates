pub mod common;
pub mod core;
pub mod tag;
pub mod url;
pub mod variable;

pub const START_TAG_LEN: usize = 2;
const END_TAG_LEN: usize = 2;

const START_TRANSLATE_LEN: usize = 2;
const END_TRANSLATE_LEN: usize = 1;
const QUOTE_LEN: usize = 1;
