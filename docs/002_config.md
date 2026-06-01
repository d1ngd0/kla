# Configuration

Kla searches for the "main" configuration file in the following places:

- `./kla.toml`
- `./.kla.toml`
- `~/.kla.toml`
- `~/.config/kla/config.toml` _preferred_
- `/etc/kla/config.toml`

The first file that it finds will be selected and parsed. If there is an error during parsing kla will return that error and stop executing.

Of course if you want to tell kla what config file it should use feel free to pass in the `--config` flag.

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

> [!NOTE]
> Relative paths are resolved based on the location of the config file they are defined in.

## Available Configurations

Below is a fully inclusive config file, with all the values specified and comments to boot!

```toml
# Sets default values for kla. These are used when not provided via flags
[default]
# Sets the implicit environment for kla, aka, the environment used if one is not
# specified
environment = ".env"

# You can specify multiple configuration directories to use at runtime as well. The
# [[config]] table has two values:
#
# - dir: A directory where all toml will be merged into the config
# - path: A single file to be loaded into config
#
# only one of these values can be used in a single definition
# Child directories **can** specify further [[config]] attributes
# 
# [[config]]
#   path = "/etc/kla/second_file.toml"
# 
# [[config]]
#   dir = "/etc/kla/config.d"
[[config]]
type = "dir"
path = "conf.d/"

# Each environment is specified in it's own environment table with
# it's name as the key (here "env_name"). It is best practice to break
# out environments (or related environments) into a separate file referenced
# by a [[config]] attribute.
[[environment]]
  # The name of the environment, this will be used to select the environment
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
  template_dir = "tmpls/env_name/"

  # The environment allows you to specify default settings when making an http
  # request. The default values for each item are commented out below.
  [environment.settings]
  # Agent specifies value we pass for the http header agent, The default value
  # can be seen below
  #agent = "kla"

  # timeout is how long we are willing for the whole request to finish.
  #
  # Units are:
  # `y`: Year. [“y” | “year” | “Y” | “YEAR” | “Year”]. e.g. 1y
  # `mon`: Month. [“mon” | “MON” | “Month” | “month” | “MONTH”]. e.g. 1mon
  # `w`: Week. [“w” | “W” | “Week” | “WEEK” | “week”]. e.g. 1w
  # `d`: Day. [“d” | “D” | “Day” | “DAY” | “day”]. e.g. 1d
  # `h`: Hour. [“h” | “H” | “hr” | “Hour” | “HOUR” | “hour”]. e.g. 1h
  # `m`: Minute. [“m” | “M” | “Minute” | “MINUTE” | “minute” | “min” | “MIN”]. e.g. 1m
  # `s`: Second. [“s” | “S” | “Second” | “SECOND” | “second” | “sec” | “SEC”]. e.g. 1s
  # `ms`: Millisecond. [“ms” | “MS” | “Millisecond” | “MilliSecond” | “MILLISECOND” | “millisecond” | “mSEC” ]. e.g. 1ms
  # `µs`: Microsecond. [“µs” | “µS” | “µsecond” | “us” | “uS” | “usecond” | “Microsecond” | “MicroSecond” | “MICROSECOND” | “microsecond” | “µSEC”]. e.g. 1µs
  # `ns`: Nanosecond. [“ns” | “NS” | “Nanosecond” | “NanoSecond” | “NANOSECOND” | “nanosecond” | “nSEC”]. e.g. 1ns
  #
  #timeout = "15m"

  # http_version is the http version we will send the request as.
  #http_version = "1.1"

  # Enable auto gzip decompression by checking the Content-Encoding response header.
  # 
  # If auto gzip decompression is turned on:
  # 
  # - When sending a request and if the request’s headers do not already
  #   contain an Accept-Encoding and Range values, the Accept-Encoding header
  #   is set to gzip. The request body is not automatically compressed.
  #
  # - When receiving a response, if its headers contain a Content-Encoding
  #   value of gzip, both Content-Encoding and Content-Length are removed
  #   from the headers’ set. The response body is automatically decompressed.
  #no_gzip=false

  # If auto brotli decompression is turned on:
  # 
  # - When sending a request and if the request’s headers do not already contain an Accept-Encoding and Range values, the Accept-Encoding header is set to br. The request body is not automatically compressed.
  # - When receiving a response, if its headers contain a Content-Encoding value of br, both Content-Encoding and Content-Length are removed from the headers’ set. The response body is automatically decompressed.
  # 
  #no_brotli=false

  # Enable auto deflate decompression by checking the Content-Encoding response header.
  # 
  # If auto deflate decompression is turned on:
  # 
  # - When sending a request and if the request’s headers do not already contain an Accept-Encoding and Range values, the Accept-Encoding header is set to deflate. The request body is not automatically compressed.
  # - When receiving a response, if it’s headers contain a Content-Encoding value that equals to deflate, both values Content-Encoding and Content-Length are removed from the headers’ set. The response body is automatically decompressed.
  #no_deflate=false

  # Max redirects defines the maximum number of redirects we are willing to follow
  # to complete the request
  #max_redirects=10

  # When no_redirects is true we refuse to follow any redirects
  #no_redirects=false

  # Connection timeout specifies how long we are willing to wait
  # for the connection to be established before we time out.
  #connect_timeout=5s

  # Accept Invalid Certs will ignore the verification of SSL Certificates.
  # Prerequisite: This is dangerous, don't configure this as true unless
  # you are in a testing environment blah blah blah
  #accept_invalid_certs=false

  # Disables hostname verification, again only use this if you know
  # what you are doing
  #accept_invalid_hostnames=false
```
