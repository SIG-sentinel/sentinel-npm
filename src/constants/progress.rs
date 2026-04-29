pub const CHECK_MAX_CONCURRENCY: usize = 20;
pub const INSTALL_MAX_CONCURRENCY: usize = 10;

pub const PROGRESS_TICK_MS: u64 = 80;
pub const PROGRESS_TICK_CHARS: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ";
pub const CHECK_PROGRESS_TEMPLATE: &str = "  {spinner:.cyan} {msg:<20} [{wide_bar:.cyan/blue}] {pos:>4}/{len:<4} elapsed {elapsed_precise}";
pub const CHECK_PROGRESS_VERIFY_MSG: &str = "verifying packages";
pub const INSTALL_PROGRESS_TEMPLATE: &str = "  {spinner:.cyan} {msg:<24} [{wide_bar:.cyan/blue}] {pos:>4}/{len:<4} elapsed {elapsed_precise} eta {eta_precise}";

pub const INSTALL_PROGRESS_VERIFY_MSG: &str = "downloading & verifying";
pub const INSTALL_PROGRESS_CACHE_MSG: &str = "installing from verified cache";
pub const INSTALL_PROGRESS_SINGLE_STEP: usize = 1;
pub const INSTALL_ALL_PACKAGES_SENTINEL: &str = "__all__";
