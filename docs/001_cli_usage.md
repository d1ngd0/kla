# CLI Usage

Kla intends to make interacting with HTTP APIs as easy as possible. There are multiple examples of documentation specifying shorthand http requests that look something like this:

> Create a notification:
> API `GET /api/v1/notify 'my notification message'`

Kla aims to make this a possibility by utilizing this exact structure, in fact putting `kla` in front of that in the terminal would run the http request. The following rules define how kla interprets your request.

With one argument Kla assumes a `GET` request, and no body:

```bash
kla /_cat/nodes
# is equivalent to
curl 'http://myenvironment.example.com/_cat/nodes'
```

With two arguments, Kla assumes the first is a method, and the second is the uri:

```bash
kla post /myindex/_rollover
# is equivalent to
curl -X POST 'http://myenvironment.example.com/myindex/_rollover'
```

Finally with three arguments, the assumption is method, uri and body:
```bash
kla post /myindex/_settings '{ "persistent" : { "cluster.routing.allocation.exclude._ip" : "10.0.0.1" } }'
# is equivalent to
curl -X POST 'http://myenvironment.example.com/myindex/_rollover' --data-binary '{ "persistent" : { "cluster.routing.allocation.exclude._ip" : "10.0.0.1" } }'
```

The body can also be preceded by an `@` symbol to denote a filepath, or a `-` to tell kla to read from standard in!

```bash
# Create a file with some body you want to send
echo '{
  "persistent" : {
    "cluster.routing.allocation.exclude._ip" : "10.0.0.1"
  }
}' > /tmp/my_settings.json

# specify the contents through stdin
cat /tmp/my_settings.json | kla post /myindex/_settings '-'

# specify the contents with a filepath
kla post /myindex/_settings '@'
```
## Environments

Kla utilizes predefined environments (much like kubectl contexts) to specify what restful endpoint you are working with. There are a few different ways to specify the environment when making a request.

### Explicit Reference

Explicit references use the name of the environment as the first item **without** a preceding `/`. This lets kla know you are specifying the endpoint to use, and not specifying part of the url. This value takes the highest precedence.

```
$ kla poetry/authors
```

### Flag Reference

Here you use the `--env` flag to specify the environment you are working with. This takes precedence over implicit, but not over explicit environment specification.

```
$ kla --env poetry /authors
```

### Implicit Reference

The final way to specify your environment is by relying on the currently configured "default environment". You can see what this value is by running `kla env` or `kla environment`. Additionally you can switch between implicit environments with `kla switch`

```
$ kla switch poetry
Switched to environment poetry
$ kla /authors
```

The argument given to switch is an optional regex. If the pattern matches only 1 environment it will be selected. If it matches multiple environments a fuzzy finder interface will help you select.

Running just `kla switch` without a pattern will present you with every environment to select from.
