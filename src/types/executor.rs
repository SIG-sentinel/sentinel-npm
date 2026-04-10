use super::install::CleanInstallPlanParams;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPlan {
    pub program: &'static str,
    pub args: Vec<String>,
}

pub trait InstallExecutor {
    fn generate_lockfile_plan(&self) -> CommandPlan;
    fn resolve_package_lockfile_plan(&self, package_reference: &str) -> CommandPlan;
    fn install_package_plan(&self, package_reference: &str, ignore_scripts: bool) -> CommandPlan;
    fn clean_install_plan(&self, params: CleanInstallPlanParams) -> CommandPlan;
}