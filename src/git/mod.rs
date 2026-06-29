mod branch;
mod command;
mod diff;
mod stage;

pub use branch::current_branch;
pub use command::run;
pub use diff::{staged_diff, staged_diff_stat, staged_files};
pub use stage::stage_all;
