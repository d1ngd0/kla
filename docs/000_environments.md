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

[[environment.settings]]
  # Sets the `User-Agent` header to be used by this client.
  agent = "some value"

  # Timeout, configuration defined by https://docs.rs/duration-string/latest/duration_string/index.html
  timeout = "10s"

  # basic authentication, with the `username:password` format. This value can be passed
  # as a raw value, or can be stored in a file, or passed in via stdint
  # `-` => Read from standard input
  # `@` => Read from file (followed by a path `@/tmp/myfile.txt`), if the file doesn't
  #        exist we do not set the value
  # `!` => Read from file (followed by a path `!/tmp/myfile.txt`), if the files doesn't
  #        exists return an error
  # .*  => Read anything else in as the literal value
  basic_auth = "username:password"

  # bearer token sets the bearer token for any requests made. This option uses the same
  # file reading features as basic_auth
  bearer_token = "h293uhfnowldkvjsdfv"

  # Set the HTTP Version, allowed versions are: 0.9, 1.0, 1.1, 2.0, 3.0.
  http_version = "1.0"

  # disable gzip
  no_gzip = false

  # disable brotli
  no_brotli = false

  # disable deflate
  no_deflate = false

  # Specify the maximum number of redirects to follow before returning an error
  # default is 10
  max_redirects = 10

  # disable redirects all together
  no_redirects = true

  # Proxy settinsg
  proxy = "http://example.com"
  proxy_http = "http://insecure.example.com"
  proxy_https = "https://secure.example.com"
  # proxy auth supports the same features as basic_auth above
  proxy_auth = "@/path/to/file"

  # Set the connection timeout?
  connect_timeout = "10s"

  # specify sigv4 settings for more information see [docs/050_aws_settings.md]
  sigv4 = true
  sigv4_aws_profile = "default"

  # do not validate certificates, don't do this unless it is a last resort.
  accept_invalid_certs = true

  # do not validate hostanmes, don't do this unless it is a last resort.
  accept_invalid_hostnames = true

  # Certificates
  certificate = "/path/to/certificate.crt"
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
