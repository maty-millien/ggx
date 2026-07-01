mod branch;
mod command;
mod commit;
mod diff;
mod push;
mod stage;

pub use branch::{current_branch, repo_root, upstream_branch};
pub use command::run;
pub use commit::commit;
pub use diff::{
    staged_diff, staged_diff_stat, staged_files, staged_numstat, unstaged_diff, unstaged_diff_stat,
    unstaged_numstat, working_tree_status,
};
pub use push::push;
pub use stage::stage_all;
