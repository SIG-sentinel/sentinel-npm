use crate::constants::{
    NPM_ARG_ADD, NPM_ARG_CI, NPM_ARG_EXACT, NPM_ARG_FROZEN_LOCKFILE, NPM_ARG_IGNORE_SCRIPTS,
    NPM_ARG_INSTALL, NPM_ARG_LOCKFILE_ONLY, NPM_ARG_MODE_UPDATE_LOCKFILE, NPM_ARG_NO_AUDIT,
    NPM_ARG_NO_FUND, NPM_ARG_OMIT_DEV, NPM_ARG_OMIT_OPTIONAL, NPM_ARG_PACKAGE_LOCK_ONLY,
    NPM_ARG_PROD, NPM_ARG_REPORTER_SILENT, NPM_ARG_SAVE_EXACT, NPM_ARG_SILENT,
};
use crate::types::{CleanInstallPlanParams, CommandPlan, InstallExecutor};

use super::PackageManager;

pub use crate::types::PackageManagerExecutor;

impl PackageManagerExecutor {
    pub fn new(manager: PackageManager) -> Self {
        Self { manager }
    }

    fn build_plan(&self, args: Vec<String>) -> CommandPlan {
        CommandPlan {
            program: self.manager.command(),
            args,
        }
    }

    fn stringify_args<'a>(args: impl IntoIterator<Item = Option<&'a str>>) -> Vec<String> {
        args.into_iter()
            .flatten()
            .map(ToString::to_string)
            .collect()
    }
}

impl InstallExecutor for PackageManagerExecutor {
    fn generate_lockfile_plan(&self) -> CommandPlan {
        let args = match self.manager {
            PackageManager::Npm => Self::stringify_args([
                Some(NPM_ARG_INSTALL),
                Some(NPM_ARG_PACKAGE_LOCK_ONLY),
                Some(NPM_ARG_IGNORE_SCRIPTS),
                Some(NPM_ARG_NO_AUDIT),
                Some(NPM_ARG_NO_FUND),
            ]),
            PackageManager::Yarn => Self::stringify_args([
                Some(NPM_ARG_INSTALL),
                Some(NPM_ARG_MODE_UPDATE_LOCKFILE),
                Some(NPM_ARG_IGNORE_SCRIPTS),
                Some(NPM_ARG_SILENT),
            ]),
            PackageManager::Pnpm => Self::stringify_args([
                Some(NPM_ARG_INSTALL),
                Some(NPM_ARG_LOCKFILE_ONLY),
                Some(NPM_ARG_IGNORE_SCRIPTS),
            ]),
        };

        self.build_plan(args)
    }

    fn resolve_package_lockfile_plan(&self, package_reference: &str) -> CommandPlan {
        let args = match self.manager {
            PackageManager::Npm => Self::stringify_args([
                Some(NPM_ARG_INSTALL),
                Some(package_reference),
                Some(NPM_ARG_SAVE_EXACT),
                Some(NPM_ARG_PACKAGE_LOCK_ONLY),
                Some(NPM_ARG_IGNORE_SCRIPTS),
                Some(NPM_ARG_NO_AUDIT),
                Some(NPM_ARG_NO_FUND),
            ]),
            PackageManager::Yarn => Self::stringify_args([
                Some(NPM_ARG_ADD),
                Some(package_reference),
                Some(NPM_ARG_EXACT),
                Some(NPM_ARG_MODE_UPDATE_LOCKFILE),
                Some(NPM_ARG_IGNORE_SCRIPTS),
            ]),
            PackageManager::Pnpm => Self::stringify_args([
                Some(NPM_ARG_ADD),
                Some(package_reference),
                Some(NPM_ARG_SAVE_EXACT),
                Some(NPM_ARG_LOCKFILE_ONLY),
                Some(NPM_ARG_IGNORE_SCRIPTS),
            ]),
        };

        self.build_plan(args)
    }

    fn install_package_plan(&self, package_reference: &str, ignore_scripts: bool) -> CommandPlan {
        let args = match self.manager {
            PackageManager::Npm => Self::stringify_args([
                Some(NPM_ARG_INSTALL),
                Some(package_reference),
                Some(NPM_ARG_SAVE_EXACT),
                Some(NPM_ARG_NO_AUDIT),
                Some(NPM_ARG_NO_FUND),
                ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
            ]),
            PackageManager::Yarn => Self::stringify_args([
                Some(NPM_ARG_ADD),
                Some(package_reference),
                Some(NPM_ARG_EXACT),
                ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
            ]),
            PackageManager::Pnpm => Self::stringify_args([
                Some(NPM_ARG_ADD),
                Some(package_reference),
                Some(NPM_ARG_SAVE_EXACT),
                ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
            ]),
        };

        self.build_plan(args)
    }

    fn clean_install_plan(&self, params: CleanInstallPlanParams) -> CommandPlan {
        let CleanInstallPlanParams {
            ignore_scripts,
            omit_dev,
            omit_optional,
            silent_output,
        } = params;

        let args = match self.manager {
            PackageManager::Npm => Self::stringify_args([
                Some(NPM_ARG_CI),
                Some(NPM_ARG_NO_AUDIT),
                Some(NPM_ARG_NO_FUND),
                omit_dev.then_some(NPM_ARG_OMIT_DEV),
                omit_optional.then_some(NPM_ARG_OMIT_OPTIONAL),
                ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
                silent_output.then_some(NPM_ARG_SILENT),
            ]),
            PackageManager::Yarn => {
                let _ = (omit_dev, omit_optional);

                Self::stringify_args([
                    Some(NPM_ARG_INSTALL),
                    Some(NPM_ARG_FROZEN_LOCKFILE),
                    ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
                    silent_output.then_some(NPM_ARG_SILENT),
                ])
            }
            PackageManager::Pnpm => {
                let _ = omit_optional;

                Self::stringify_args([
                    Some(NPM_ARG_INSTALL),
                    Some(NPM_ARG_FROZEN_LOCKFILE),
                    ignore_scripts.then_some(NPM_ARG_IGNORE_SCRIPTS),
                    silent_output.then_some(NPM_ARG_REPORTER_SILENT),
                    omit_dev.then_some(NPM_ARG_PROD),
                ])
            }
        };

        self.build_plan(args)
    }
}
