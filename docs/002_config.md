# Configuration

Kla searches for the "main" configuration file in the following places:

- `config.toml`
- `~/.kla.toml`
- `~/.config/kla/config.toml` _prefered_
- `/etc/kla/config.toml`

The first file that it finds will be selected and parsed. If there is an error during parsing kla will return that error and stop executing.

## Additional Configuration Files

Configuration can be broken into multiple files! Your "main" config, **and only your main config**, can specify a `[[config]]` attribute which pulls in additional directories or paths.

```toml
[[config]]
type = "file"
path = "/etc/kla/elasticsearch_environment.toml"

[[config]]
type = "file"
path = "/etc/kla/ntfy.toml"
```

You can also specify a directory!

```toml
[[config]]
type = "dir"
path = "/etc/kla/conf.d"
```

Specifying both a `dir` and `path` will result in an error, so don't do that. The only difference between your "main" config file and others is that only "main" can have the `[[config]]` attribute, it will be ignored in any merged files.

## Available Configurations

Below is a fully inclusive config file, with all the values specified and comments to boot!

```toml
# Sets default values for kla. These are used when not provided via flags
[default]
# sets the --env flag by default. This value is actually a file since the `switch`
# subcommand can change this value to switch contexts.
environment = "~/.default-environment"

# You can specify multiple configuration directories to use at runtime as well. The
# [[config]] table has two values:
# dir: A directory where all toml will be merged into the config
# path: A single file to be loaded into config
# only one of these values can be used in a single definition
# Child directories **can not** specify further [[config]] attributes
# 
# toml
# # valid
# [[config]]
#   path = "/etc/kla/second_file.toml"
# 
# # valid
# [[config]]
#   dir = "/etc/kla/config.d"
# 
# # invalid
# [[config]]
#   dir = "/etc/kla/config.d"
#   path = "/etc/kla/second_file.toml"
# 
# # invalid
# [[config]]
#   my_random_attribute = "something"
[[config]]
type = "dir"
path = "~/.config/kla/conf.d/"

# Each environment is specified in it's own environment table with
# it's name as the key (here "env_name"). It is best practice to break
# out environments (or related environments) into a separate file referenced
# by a [[config]] attribute.
[[environment]]
  # The name of the environment, this will be used when selecting a value
  name = "env_name"

  # The url is the prefix for any http requests we build from it. If there
  # is a specific port, scheme, or path prefix include it here.
  # feel free to add or omit the trailing slash :)
  url = "http://example.com:9999/api/v1"

  # Provides a short description of the environment, used when listing
  # environments
  short_description = "An example API"

  # The long description is used in the fuzzy finder preview window.
  long_description = """
A much longer description"""

  # template_dir is where the templates are stored. All top level files within
  # this directory are parsed and turned into subcommands under `kla run`.
  # Checkout https://github.com/d1ngd0/kla/blob/main/docs/003_templates.md for more
  # information on templates specifically.
  template_dir = "~/.config/kla/tmpls/env_name/"

  # Specifies the --sigv4 flag should be enabled signing the http request with
  # amazons sigv4 https://docs.aws.amazon.com/IAM/latest/UserGuide/reference_sigv.html
  # sigv4_aws_profile specifies the profile to use and sigv4_aws_service defines the
  # service to use.
  # These settings assume you have set up your AWS credentials correctly. see
  # https://github.com/d1ngd0/kla/blob/main/docs/050_aws_settings.md
  sigv4 = true
  sigv4_aws_profile = "default"
  sigv4_aws_service = "execute-api"
```
