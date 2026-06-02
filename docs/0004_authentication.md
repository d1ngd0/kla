# Authentication

While many authentication endpoints can be implemented with bispoke kla templates, many authentication mechanisms are built-in to make interacting with a RESTful API easier. The following authentication flows are supported:

- [Basic Authentication](#basic_authentication)
- [Bearer Token](#bearer_token)
- [OAuth](#oauth)
- [ROPC](#ropc)
- [SigV4](#sigv4)

An environment can be configured with a specific authentication mechanism, and kla will prompt you for the things it needs on your first query.

## SecretValue

Some of the fields in kla configs are SecretValues, which can be set in
multiple ways:

- A literal value
- A Filepath, with the value stored in that file
- An Environment Variable
- Prompting the user for the value.

Using `password` as the example, each type would be configured thus:

```toml
# Literal value
password = "hunter02"

# File Path
# `path` defines a path to the file with the password in it. Relative paths are resolved from the location of the file referencing it.
# `trim` _optional_ is used to trim the value within the file, removing any trailing whitespace from the files content
password = { path = "/etc/kla/super-secure-password", trim = true }

# Environment Variable
# `env` specifies the environment variable to collect the value from
# `default` _optional_ specifies the default value to use if the variable is not set
# If the value is not set, and there is no default specified, and error will be returned stating the variable is not set
password = { env = "KLA_ENVIRONMENT_PASSWORD" }

# Prompting
# `prompt` defines the prompt you want to show the user to collect the
# secret value. The value they type in will be masked.
password = { prompt = "Users Password" }
```

# Basic

_type: `basic`_

[Basic Authentication](https://en.wikipedia.org/wiki/Basic_access_authentication)

_environment configuration example_

```toml
[environment.settings.auth]
  type = "basic"

  # username is the username of the user to basically authenticate.
  # There are a few. This field is a SecretValue 
  # required
  # When `prompt` is used, the user will be prompted every time
  username = "username"

  # password is the password for the user. This field is a SecretValue
  # Optional
  # The user will be prompted every time a request is made.
  password = { prompt = "Basic Authentication Password" }
```

## Bearer Token

_type: `bearer`_

A bearer token sets the authentication header with the specified bearer token.

_environment configuration example_

```toml
[environment.settings.auth]
  type = "bearer"

  # Token specifies the oauth token to use, This field is a SecretValue.
  # When using `prompt` the user will be prompted for the token for every
  # request.
  token = { path = ".oauth_token.jwt", trim = true }
```

## OAuth

_type: `oauth`_

Creates a [Oauth 2.0](https://oauth.net/2/) flow, redirecting the user to a browser to authenticate. The user **will not** be prompted to follow the flow on every request, only when their cached token has expired.

The `redirect_url` must be to `{http|https}://127.0.0.1:{redirect_port}` with no `/` postfix. When using https kla will generate self signed certificates for you.

```toml
[environment.settings.auth]
  type = "oauth"

  # client_id specifies the oauth client id. This field is a SecretValue
  # when configured with `prompt` the user will be prompted after once
  # their token has expired
  # required
  client_id = "kla_id"

  # client_secret specifies the oauth client secret. This field is a
  # SecretValue when configured with `prompt` the user will be prompted
  # after once their token has expired
  # required
  client_secret = "super_secret_value"

  # authorization_url is the location we open a browser to for login
  # required
  authorization_url = "https://oauth.example.com/oauth/v2/authorize"

  # redirect_port specifies the port open from within kla to capture
  # the redirection after authorization.
  # required
  redirect_port = 8080

  # https creates a redirect url with https instead of the default
  # http
  # optional
  https = false

  # token_url is where kla will fetch the token after authorization
  token_url = "https://oauth.example.com/oauth/v2/token"

  # scopes a
  scopes = ["read", "write"]
```

## ROPC

ROPC is an older version of OAuth for system to system authentication. It is best practice to move away from this authentication mechanism. **But there is a lot of this out in the wild** and it is better than nothing, so we got you.


```toml
[environment.settings.auth]
  type = "ropc"

  # client_id specifies the oauth client id. This field is a SecretValue
  # when configured with `prompt` the user will be prompted after once
  # their token has expired
  # required
  client_id = "kla_id"

  # client_secret specifies the oauth client secret. This field is a
  # SecretValue when configured with `prompt` the user will be prompted
  # after once their token has expired
  # required
  client_secret = "super_secret_value"

  # token_url is where kla will fetch the token after authorization
  token_url = "https://oauth.example.com/oauth/v2/token"

  # username is the username for the ropc user. This field is a
  # SecretValue when configured with `prompt` the user will be prompted
  # after once their token has expired
  # required
  username = "kla_ropc"

  # password is the password for the ropc user. This field is a
  # SecretValue when configured with `prompt` the user will be prompted
  # after once their token has expired
  # required
  password = "hunter02"
```

## SigV4


In order to use AWS Sigv4 with your requests you need to appropriately configure your machine. whether you have multiple AWS keys or not kla has your back.

KLA assumes you have set up your [Configuration and Credentials](https://docs.aws.amazon.com/cli/v1/userguide/cli-configure-files.html) files, but if not here is what you need.

In your `~/.aws/credentials` file you will want to add any keys you currently have

```toml
[default]
aws_access_key_id=ASIAIOSFODNN7EXAMPLE
aws_secret_access_key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
aws_session_token = IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZVERYLONGSTRINGEXAMPLE

[user1]
aws_access_key_id=ASIAI44QH8DHBEXAMPLE
aws_secret_access_key=je7MtGbClwBF/2Zp9Utk/h3yCo8nvbEXAMPLEKEY
aws_session_token = fcZib3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZ2luX2IQoJb3JpZVERYLONGSTRINGEXAMPLE
```

Within your `~/.aws/config` file you will want the appropriate settings for region.

```toml
[default]
region=us-west-2
output=json

[profile user1]
region=us-east-1
output=text
```

Finally your environment is configured as such

```toml
[environment.settings.auth]
  type = "sigv4"

  # sigv4_aws_profile specifies the profile to use and sigv4_aws_service defines the
  # service to use.
  # These settings assume you have set up your AWS credentials correctly.
  sigv4_aws_profile = "user1"
```
