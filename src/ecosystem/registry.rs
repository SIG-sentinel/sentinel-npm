use std::future::Future;
use std::pin::Pin;

use crate::npm::NpmRegistry;
use crate::types::{NpmVersionMeta, PackageRef, SentinelError};

pub trait RegistryClient {
    fn fetch_version<'a>(
        &'a self,
        package_ref: &'a PackageRef,
    ) -> Pin<Box<dyn Future<Output = Result<NpmVersionMeta, SentinelError>> + Send + 'a>>;
}

impl RegistryClient for NpmRegistry {
    fn fetch_version<'a>(
        &'a self,
        package_ref: &'a PackageRef,
    ) -> Pin<Box<dyn Future<Output = Result<NpmVersionMeta, SentinelError>> + Send + 'a>> {
        Box::pin(async move { self.fetch_version(package_ref).await })
    }
}
