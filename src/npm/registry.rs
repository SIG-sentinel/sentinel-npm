use std::time::Duration;

use crate::constants::{
    DOWNLOAD_TARBALL_TIMEOUT_SECS, NPM_ERR_PARSE_RESPONSE_TEMPLATE,
    NPM_ERR_REGISTRY_RESPONSE_TEMPLATE, NPM_ERR_TIMEOUT_DOWNLOAD_TEMPLATE, NPM_REGISTRY_BASE_URL,
    NPM_SCOPED_SEPARATOR, NPM_USER_AGENT_PREFIX, render_template,
};
use crate::types::{NpmVersionMeta, PackageRef, SentinelError};

pub use crate::types::NpmRegistry;

impl NpmRegistry {
    pub fn new(timeout_ms: u64) -> Result<Self, SentinelError> {
        let client = reqwest::Client::builder()
            .user_agent(format!(
                "{}{}",
                NPM_USER_AGENT_PREFIX,
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(Duration::from_millis(timeout_ms))
            .https_only(true)
            .use_rustls_tls()
            .build()
            .map_err(|error| SentinelError::Http(error.to_string()))?;

        Ok(Self {
            client,
            timeout: Duration::from_millis(timeout_ms),
        })
    }

    pub async fn fetch_version(
        &self,
        package_ref: &PackageRef,
    ) -> Result<NpmVersionMeta, SentinelError> {
        let url = format!(
            "{}/{}/{}",
            NPM_REGISTRY_BASE_URL,
            encode_package_name(&package_ref.name),
            package_ref.version
        );

        let resp = tokio::time::timeout(self.timeout, self.client.get(&url).send())
            .await
            .map_err(|_| SentinelError::RegistryTimeout {
                package: package_ref.name.clone(),
                version: package_ref.version.clone(),
                ms: self.timeout.as_millis() as u64,
            })?
            .map_err(SentinelError::from)?;

        let status_code = resp.status().as_u16();
        let status_text = resp.status().to_string();
        let is_not_found = status_code == 404;
        let is_success = resp.status().is_success();

        match (is_not_found, is_success) {
            (true, _) => Err(SentinelError::NoIntegrity {
                package: package_ref.name.clone(),
                version: package_ref.version.clone(),
            }),
            (_, false) => Err(SentinelError::Http(render_template(
                NPM_ERR_REGISTRY_RESPONSE_TEMPLATE,
                &[status_text, package_ref.to_string()],
            ))),
            (_, true) => resp.json::<NpmVersionMeta>().await.map_err(|error| {
                SentinelError::Http(render_template(
                    NPM_ERR_PARSE_RESPONSE_TEMPLATE,
                    &[package_ref.to_string(), error.to_string()],
                ))
            }),
        }
    }

    pub async fn download_tarball(&self, url: &str) -> Result<reqwest::Response, SentinelError> {
        tokio::time::timeout(
            Duration::from_secs(DOWNLOAD_TARBALL_TIMEOUT_SECS),
            self.client.get(url).send(),
        )
        .await
        .map_err(|_| {
            SentinelError::Http(render_template(
                NPM_ERR_TIMEOUT_DOWNLOAD_TEMPLATE,
                &[url.to_string()],
            ))
        })?
        .map_err(SentinelError::from)
    }
}

fn encode_package_name(name: &str) -> String {
    match name.starts_with('@') {
        true => name.replacen('/', NPM_SCOPED_SEPARATOR, 1),
        false => name.to_string(),
    }
}
