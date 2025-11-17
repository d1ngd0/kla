# Environments 

Kla is built around environments. When using `curl` you need to specify a fully qualified domain and uri to pull down what you want.

```bash
curl 'http://example.com/api/message' --data-binary 'This is my message' -X PUT
```

With kla, when the path is not a fully qualified domain we fall back on the current environment. When one is selected we can accomplish the same request with the following:

```bash
kla put /api/message 'This is my message'
```

## Defining an Environment

All environments are configured in the [Configuration File](002_config.md) like so:

```toml
[[environment]]
name = "example"
url = "http://example.com/"
```

<sub>_The only required attribute is the `url`, but there are plenty more configurations_</sub>

You can then run `kla switch example` to select example as your environment.

## Fuzzy Selection of an environment

You might not totally remember what you named the environment, and that is OK. Running `kla switch` without an argument will bring up a fuzzy finder to help find the right environment.

<sub>_If you only have one environment it will be auto selected</sub>

## Seeing your environments

You can see all of your current environments with `kla environments`.

## Explicitly specifying your environment

You can be explicit about what environment you want to be running as with the `--env` flag

```
kla --env poetry /authors
```
