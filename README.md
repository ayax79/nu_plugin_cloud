# nu_plugin_cloud

Provides uniform access to cloud storage services for nushell.

# Features
- `cloud ls` - List the filenames, sizes, modificationtime , etags, and versions of a cloud location.
- `cloud open` - Load a file into a cell, converting to table if possible (avoid by appending '--raw').
- `cloud rm` - Remove a file from cloud sotrage
- `cloud save` - Save a file to cloud storage
- AWS S3 support
- Coming Soon: Azure support
- Coming Soon: Google cloud support

## Installation

### Prerequisites
- Install [rustup](https://rustup.rs/)
- For AWS SSO support, install the [AWS CLI](https://aws.amazon.com/cli/)

### Installation From Source
This will be published on crates.io once it is in a more complete state. For now:
```nushell
git clone https://github.com/ayax79/nu_plugin_cloud.git
cd nu_plugin_cloud
cargo install --path .
plugin add ~/.cargo/bin/nu_plugin_cloud
plugin use cloud
```

## AWS Setup

Configuration for AWS uses the standard [Configuration and Credential Files](https://docs.aws.amazon.com/cli/v1/userguide/cli-configure-files.html). To change your profile, ensure that the AWS_PROFILE environment variable is set to the desired profile.

### AWS SSO

For SSO, the AWS CLI is required to configure and login. To setup AWS SSO:
- follow the steps on [Configure the AWS CLI with IAM identity Center authentication](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html).
- login with `aws sso login`

This plugin uses the aws_config crate. It is very unforgiving of miss configurations.

It will work if the profile includes only the sso_session, sso_account_id, and sso_role_name. Don't include sso_start_url in the profile section for instance.

```
[profile my-profile]
sso_session = my-sso
sso_account_id = <numeric account id>
sso_role_name = my-iam-role
region = us-east-1
output = json

[sso-session disqo]
sso_start_url = https://d-92677e5ab0.awsapps.com/start
sso_region = us-west-2
sso_registration_scopes = sso:account:access
```
