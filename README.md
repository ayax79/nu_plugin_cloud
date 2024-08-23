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

### Installation With Cargo
```nu
cargo install nu_plugin_cloud
plugin add ~/.cargo/bin/nu_plugin_cloud
plugin use cloud
```

### Installation From Source
```nu
git clone https://github.com/ayax79/nu_plugin_cloud.git
cd nu_plugin_cloud
cargo install --path .
plugin add ~/.cargo/bin/nu_plugin_cloud
plugin use cloud
```

> [!TIP]
> This plugin will turn of plugin GC when any operation happens. 

To turn plugin GC back off and query internal caches, run:
```nu
cloud cache-clear
```

# AWS Support

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

[sso-session my-sso]
sso_start_url = https://d-92677e5ab0.awsapps.com/start
sso_region = us-west-2
sso_registration_scopes = sso:account:access
```
## Non-Cloud Storage

There are two types of supported non-cloud storage types, in-memory and file system. It can be useful to use these for testing purposes.

### In-Memory Usage

Save a file from memory:
```nu
[[a b]; [1 2]] | cloud save memory:/foo.csv
```

Load a file from memory:
```nu
cloud open memory:/foo.csv
```

List files in memory:
```nu
cloud ls memory:/foo.csv
```

### Filesystem Usage

Save a file from the local filesystem:
```nu
[[a b]; [1 2]] | cloud save file:///tmp/test/foo.csv
```

Load a file from the local filesystem:
```nu
cloud open file:///tmp/test/foo.csv
```

List files in the local filesystem:
```nu
cloud ls file:///tmp/test/foo.csv
```
