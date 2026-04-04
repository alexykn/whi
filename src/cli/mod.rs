pub mod args;
pub mod dispatch;

pub use args::{
    ApplyTarget, Args, ColorWhen, DeleteTarget, HistoryAction, PathEdit, PreferTarget,
    parse_add_arguments, parse_delete_arguments, parse_prefer_arguments,
};
