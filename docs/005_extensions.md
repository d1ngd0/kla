# Extensions

You may have a collection of templates and environments that you want to share with others. Instead of sharing files with one another that may quickly fall out of date you can package your configurations up into an oci image with a semver tag and install it with a simple `kla ext add example.com/something/my-endpoint`.

Extensions are just bundles of kla configuration. The only requirement is having a `config.toml` file in the root of the OCI Image. An example of an extension can be found [here](https://github.com/d1ngd0/poetry-kla).




