use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::{provider::ProvideCredentials, Credentials};
use nu_protocol::ShellError;
use object_store::aws::AmazonS3ConfigKey;

pub async fn options() -> Result<Vec<(String, String)>, ShellError> {
    let aws_config = aws_load_config().await;
    let mut options: Vec<(String, String)> = Vec::with_capacity(3);
    if let Some(region) = aws_config.region() {
        options.push((
            AmazonS3ConfigKey::Region.as_ref().to_string(),
            region.to_string(),
        ));
    }
    if let Some(credentials) = aws_creds(&aws_config).await? {
        options.push((
            AmazonS3ConfigKey::AccessKeyId.as_ref().to_string(),
            credentials.access_key_id().to_string(),
        ));
        options.push((
            AmazonS3ConfigKey::SecretAccessKey.as_ref().to_string(),
            credentials.secret_access_key().to_string(),
        ));
    }

    Ok(options)
}

async fn aws_load_config() -> SdkConfig {
    aws_config::load_defaults(BehaviorVersion::latest()).await
}

async fn aws_creds(aws_config: &SdkConfig) -> Result<Option<Credentials>, ShellError> {
    if let Some(provider) = aws_config.credentials_provider() {
        Ok(Some(provider.provide_credentials().await.map_err(|e| {
            ShellError::GenericError {
                error: format!("Could not fetch AWS credentials: {e}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }
        })?))
    } else {
        Ok(None)
    }
}
