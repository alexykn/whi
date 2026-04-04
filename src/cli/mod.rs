pub mod args;
pub mod dispatch;

pub use args::{
    parse_add_arguments, parse_delete_arguments, parse_prefer_arguments, Args, ColorWhen,
    DeleteTarget, PreferTarget,
};
