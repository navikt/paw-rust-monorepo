use crate::error::ServerError;
use anyhow::Result;

pub fn get_env(key: &'static str) -> Result<String> {
    let var = std::env::var(key).map_err(|_| ServerError::EnvVarNotFound(key.to_string()))?;
    Ok(var)
}

pub fn nais_otel_service_name() -> Result<String> {
    get_env("OTEL_SERVICE_NAME")
}

pub fn nais_namespace() -> Result<String> {
    get_env("NAIS_NAMESPACE")
}

pub const NAIS_CLUSTER_NAME: &str = "NAIS_CLUSTER_NAME";
pub fn nais_cluster_name() -> Result<String> {
    get_env(NAIS_CLUSTER_NAME)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEnv {
    ProdGcp,
    DevGcp,
    Local,
    UnknownEnv(String),
}

pub const PROD_GCP_CLUSTER_NAME: &str = "prod-gcp";
pub const DEV_GCP_CLUSTER_NAME: &str = "dev-gcp";

pub fn runtime_env() -> RuntimeEnv {
    match nais_cluster_name() {
        Ok(cluster) if cluster == PROD_GCP_CLUSTER_NAME => RuntimeEnv::ProdGcp,
        Ok(cluster) if cluster == DEV_GCP_CLUSTER_NAME => RuntimeEnv::DevGcp,
        Ok(cluster) => RuntimeEnv::UnknownEnv(cluster),
        Err(_) => RuntimeEnv::Local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returnerer_prod_nar_cluster_er_prod_gcp() {
        temp_env::with_var(NAIS_CLUSTER_NAME, Some(PROD_GCP_CLUSTER_NAME), || {
            assert_eq!(runtime_env(), RuntimeEnv::ProdGcp);
        });
    }

    #[test]
    fn returnerer_dev_nar_cluster_er_dev_gcp() {
        temp_env::with_var(NAIS_CLUSTER_NAME, Some(DEV_GCP_CLUSTER_NAME), || {
            assert_eq!(runtime_env(), RuntimeEnv::DevGcp);
        });
    }

    #[test]
    fn returnerer_local_nar_cluster_mangler() {
        temp_env::with_var_unset(NAIS_CLUSTER_NAME, || {
            assert_eq!(runtime_env(), RuntimeEnv::Local);
        });
    }

    #[test]
    fn returnerer_unknown_nais_nar_cluster_er_ukjent() {
        temp_env::with_var(NAIS_CLUSTER_NAME, Some("prod-fss"), || {
            assert_eq!(
                runtime_env(),
                RuntimeEnv::UnknownEnv("prod-fss".to_string())
            );
        });
    }
}
