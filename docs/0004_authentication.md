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
- Prompting the user for the value.

Using `password` as the example, each type would be configured thus:

```toml
# Literal value
password = "hunter02"

# File Path
# `path` defines a path to the file with the password in it. Relative paths are resolved from the location of the file referencing it.
# `trim` is used to trim the value within the file, removing any trailing whitespace from the files content
password = { path = "/etc/kla/super-secure-password", trim = true }

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

## 
