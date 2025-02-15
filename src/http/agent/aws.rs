use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Error;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_sts::Client as StsClient;
use aws_smithy_types::DateTime;
use serde::{Serialize, Serializer};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone)]
pub(crate) struct AwsState {
    sts_client: StsClient,
    credential_cache: Arc<RwLock<Vec<CachedCredential>>>,
}

impl AwsState {
    pub async fn new() -> Self {
        let region_provider = RegionProviderChain::default_provider();
        let config = aws_config::from_env().region(region_provider).load().await;
        let sts_client = StsClient::new(&config);
        let credential_cache = Arc::new(RwLock::new(vec![]));

        Self {
            sts_client,
            credential_cache,
        }
    }

    pub async fn get_credentials(
        &self,
        role: String,
        session_name: String,
    ) -> Result<TemporaryCredential, Error> {
        if let Some(creds) = self.get_cached_credential(&role).await {
            info!("using cached credentials for role {}", role);
            return Ok(TemporaryCredential {
                version: 1,
                access_key_id: creds.access_key_id,
                secret_access_key: creds.secret_access_key,
                session_token: creds.session_token,
                expiration: creds.expiration,
            });
        }
        let creds = self
            .sts_client
            .assume_role()
            .set_role_session_name(Some(session_name))
            .set_role_arn(Some(role.clone()))
            .set_duration_seconds(Some(3600))
            .send()
            .await
            .map_err(|e| Error::AwsError(e.to_string()))?;

        let creds = creds
            .credentials()
            .ok_or_else(|| Error::AwsError("credentials not provided".to_string()))?;
        let tc = TemporaryCredential {
            version: 1,
            access_key_id: creds.access_key_id().into(),
            secret_access_key: creds.secret_access_key().into(),
            session_token: creds.session_token().into(),
            expiration: creds.expiration().to_owned(),
        };

        self.add_cached_credential(CachedCredential {
            role,
            credential: tc.clone(),
        })
        .await;
        Ok(tc)
    }
    async fn get_cached_credential(&self, role: &str) -> Option<TemporaryCredential> {
        let guard = self.credential_cache.read().await;
        let now = SystemTime::now();
        for cached_cred in guard.iter() {
            if role == cached_cred.role && !expired(&cached_cred.credential.expiration, now).ok()? {
                return Some(cached_cred.credential.clone());
            }
        }
        None
    }
    async fn add_cached_credential(&self, cached_cred: CachedCredential) {
        let mut guard = self.credential_cache.write().await;
        for cred in guard.iter_mut() {
            if cred.role == cached_cred.role {
                info!("updating credentials for role {}", cached_cred.role);
                cred.credential = cached_cred.credential;
                return;
            }
        }
        drop(guard);
        info!("caching credentials for role {}", cached_cred.role);
        self.credential_cache.write().await.push(cached_cred);
    }
}

fn expired(credential_expiration: &DateTime, now: SystemTime) -> Result<bool, Error> {
    let now_as_secs = now.duration_since(UNIX_EPOCH)?.as_secs();
    let credential_expiration = credential_expiration.secs();
    let time_left = credential_expiration.checked_sub_unsigned(now_as_secs);
    if let Some(time_left) = time_left {
        Ok(time_left < 900)
    } else {
        Ok(false)
    }
}

#[derive(Debug, Clone)]
struct CachedCredential {
    role: String,
    credential: TemporaryCredential,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct TemporaryCredential {
    pub version: usize,
    pub access_key_id: String,
    pub secret_access_key: String,
    #[serde(rename = "Token")]
    pub session_token: String,
    #[serde(serialize_with = "serialize_date_time")]
    pub expiration: DateTime,
}

fn serialize_date_time<S>(dt: &DateTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(dt.to_string().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_checks() {
        let now = SystemTime::now();
        let now_as_secs = now.duration_since(UNIX_EPOCH).unwrap().as_secs() + 1;
        let dt = DateTime::from_secs(now_as_secs as i64);
        // expired check
        assert!(expired(&dt, now).unwrap());

        // not expired
        let dt = DateTime::from_secs(now_as_secs as i64 + 901);
        assert!(!expired(&dt, now).unwrap());
    }
}
