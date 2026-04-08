use crate::constants::{
    NPM_ARG_ADD, NPM_ARG_CI, NPM_ARG_EXACT, NPM_ARG_FROZEN_LOCKFILE, NPM_ARG_IGNORE_SCRIPTS,
    NPM_ARG_INSTALL, NPM_ARG_LOCKFILE_ONLY, NPM_ARG_MODE_UPDATE_LOCKFILE, NPM_ARG_NO_AUDIT,
    NPM_ARG_NO_FUND, NPM_ARG_OMIT_DEV, NPM_ARG_OMIT_OPTIONAL, NPM_ARG_PACKAGE_LOCK_ONLY,
    NPM_ARG_PROD, NPM_ARG_REPORTER_SILENT, NPM_ARG_SAVE_EXACT, NPM_ARG_SILENT,
};
use crate::types::CleanInstallPlanParams;

use super::PackageManager;

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

pub struct PackageManagerExecutor {
    manager: PackageManager,
}

impl PackageManagerExecutor {
    pub fn new(manager: PackageManager) -> Self {
        Self { manager }
    }
}

impl InstallExecutor for PackageManagerExecutor {
    fn generate_lockfile_plan(&self) -> CommandPlan {
        match self.manager {
            PackageManager::Npm => CommandPlan {
                program: self.manager.command(),
                args: vec![
                    NPM_ARG_INSTALL.to_string(),
                    NPM_ARG_PACKAGE_LOCK_ONLY.to_string(),
                    NPM_ARG_IGNORE_SCRIPTS.to_string(),
                    NPM_ARG_NO_AUDIT.to_string(),
                    NPM_ARG_NO_FUND.to_string(),
                ],
            },
            PackageManager::Yarn => CommandPlan {
                program: self.manager.command(),
                args: vec![
                    NPM_ARG_INSTALL.to_string(),
                    NPM_ARG_MODE_UPDATE_LOCKFILE.to_string(),
                    NPM_ARG_IGNORE_SCRIPTS.to_string(),
                    NPM_ARG_SILENT.to_string(),
                ],
            },
            PackageManager::Pnpm => CommandPlan {
                program: self.manager.command(),
                args: vec![
                    NPM_ARG_INSTALL.to_string(),
                    NPM_ARG_LOCKFILE_ONLY.to_string(),
                    NPM_ARG_IGNORE_SCRIPTS.to_string(),
                ],
            },
        }
    }

    fn resolve_package_lockfile_plan(&self, package_reference: &str) -> CommandPlan {
        match self.manager {
            PackageManager::Npm => CommandPlan {
                program: self.manager.command(),
                args: vec![
                    NPM_ARG_INSTALL.to_string(),
                    package_reference.to_string(),
                    NPM_ARG_SAVE_EXACT.to_string(),
                    NPM_ARG_PACKAGE_LOCK_ONLY.to_string(),
                    NPM_ARG_IGNORE_SCRIPTS.to_string(),
                    NPM_ARG_NO_AUDIT.to_string(),
                    NPM_ARG_NO_FUND.to_string(),
                ],
            },
            PackageManager::Yarn => CommandPlan {
                program: self.manager.command(),
                args: vec![
                    NPM_ARG_ADD.to_string(),
                    package_reference.to_string(),
                    NPM_ARG_EXACT.to_string(),
                    NPM_ARG_MODE_UPDATE_LOCKFILE.to_string(),
                    NPM_ARG_IGNORE_SCRIPTS.to_string(),
                ],
            },
            PackageManager::Pnpm => CommandPlan {
                program: self.manager.command(),
                args: vec![
                    NPM_ARG_ADD.to_string(),
                    package_reference.to_string(),
                    NPM_ARG_SAVE_EXACT.to_string(),
                    NPM_ARG_LOCKFILE_ONLY.to_string(),
                    NPM_ARG_IGNORE_SCRIPTS.to_string(),
                ],
            },
        }
    }

    fn install_package_plan(&self, package_reference: &str, ignore_scripts: bool) -> CommandPlan {
        match self.manager {
            PackageManager::Npm => {
                let mut args = vec![
                    NPM_ARG_INSTALL.to_string(),
                    package_reference.to_string(),
                    NPM_ARG_SAVE_EXACT.to_string(),
                    NPM_ARG_NO_AUDIT.to_string(),
                    NPM_ARG_NO_FUND.to_string(),
                ];
                if ignore_scripts {
                    args.push(NPM_ARG_IGNORE_SCRIPTS.to_string());
                }

                CommandPlan {
                    program: self.manager.command(),
                    args,
                }
            }
            PackageManager::Yarn => {
                let mut args = vec![
                    NPM_ARG_ADD.to_string(),
                    package_reference.to_string(),
                    NPM_ARG_EXACT.to_string(),
                ];

                if ignore_scripts {
                    args.push(NPM_ARG_IGNORE_SCRIPTS.to_string());
                }

                CommandPlan {
                    program: self.manager.command(),
                    args,
                }
            }
            PackageManager::Pnpm => {
                let mut args = vec![
                    NPM_ARG_ADD.to_string(),
                    package_reference.to_string(),
                    NPM_ARG_SAVE_EXACT.to_string(),
                ];

                if ignore_scripts {
                    args.push(NPM_ARG_IGNORE_SCRIPTS.to_string());
                }

                CommandPlan {
                    program: self.manager.command(),
                    args,
                }
            }
        }
    }

    fn clean_install_plan(&self, params: CleanInstallPlanParams) -> CommandPlan {
        let CleanInstallPlanParams {
            ignore_scripts,
            omit_dev,
            omit_optional,
            silent_output,
        } = params;

        match self.manager {
            PackageManager::Npm => {
                let args: Vec<String> = [
                    Some(NPM_ARG_CI),
                    Some(NPM_ARG_NO_AUDIT),
                    Some(NPM_ARG_NO_FUND),
                    omit_dev.then_some(NPM_ARG_OMIT_DEV),
                    omit_optional.then_some(NPM_ARG_OMIT_OPTIONAL),
                    ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
                    silent_output.then_some(NPM_ARG_SILENT),
                ]
                .into_iter()
                .flatten()
                .map(|s| s.to_string())
                .collect();

                CommandPlan { program: self.manager.command(), args }
            }
            PackageManager::Yarn => {
                let _ = (omit_dev, omit_optional);

                let args: Vec<String> = [
                    Some(NPM_ARG_INSTALL),
                    Some(NPM_ARG_FROZEN_LOCKFILE),
                    ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
                    silent_output.then_some(NPM_ARG_SILENT),
                ]
                .into_iter()
                .flatten()
                .map(|s| s.to_string())
                .collect();

                CommandPlan { program: self.manager.command(), args }
            }
            PackageManager::Pnpm => {
                let _ = omit_optional;

                let args: Vec<String> = [
                    Some(NPM_ARG_INSTALL),
                    Some(NPM_ARG_FROZEN_LOCKFILE),
                    ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
                    silent_output.then_some(NPM_ARG_REPORTER_SILENT),
                    omit_dev.then_some(NPM_ARG_PROD),
                ]
                .into_iter()
                .flatten()
                .map(|s| s.to_string())
                .collect();

                CommandPlan { program: self.manager.command(), args }
            }
        }
    }
}
