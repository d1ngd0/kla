# Extensions

You may have a collection of templates and environments that you want to share with others. Instead of sharing files with one another that may quickly fall out of date you can package your configurations up into an oci image with a semver tag and install it with a simple `kla ext add example.com/something/my-endpoint`.

Extensions are just bundles of kla configuration. The only requirement is having a `config.toml` file in the root of the OCI Image. An example of an extension can be found [here](https://github.com/d1ngd0/poetry-kla).

## Configuration

```toml
[extensions]
  # enables or disables merging extensions into your config. If this is set to
  # false you can still add update and remove extension, they just won't be merged
  # into your active config.
  enabled = false

  # This is the directory your extensions is stored into. This should probably be
  # left unset as it will automatically detect your OSes preferred location
  dir = ".extensions/"

  # If you need authentication you can configure your registries ahead of time
  # If the registry matches the prefix it will use the supplied authentication
  # method. For information on authentication methods check out [0004_authentication.md](0004_authentication.md).
  # Supported authentication methods are:
  # - basic
  # - bearer
  # - oauth
  # - ropc
  [[extensions.registries]]
    prefix = "docker.paulmontag.info/"
    authentication = { type = "basic", username = "paul", password = { password = "docker.paulmontag.info registry password: " } }
```
