# Templates

You may find yourself running the same command over and over again, instead of doing that, just create a template!

Each [Environment](000_environments.md) has an optional `template_dir` you can specify which houses `toml` files specifying subcommands. If you have an environment with templates already you can run `kla run --help` for a full list of what they are and what they do. Kla interprets these files at runtime to build subcommands for `run` making them highly integrated with kla for a seamless experience (enough buzz words, let's make one)

Let's assume you have the following kla commands you run ALL THE TIME and you want to turn it into a template.

```bash
kla /api/v1/doc/233
kla DELETE /api/v1/doc/233 '{"force": true}'
```

Let's start with the basics; create a file in the environments `template_dir` named `doc.toml`

```
uri = "/api/v1/doc/{{ id }}"
help = "View a document"

[[arg]]
  name = "id"
  required = true
```

With this alone we can already look at the document with the following command:

```bash
kla run doc 233
```

but we need to delete as well. For that let's add a `--delete` flag that will change the method

```toml
uri = "/api/v1/doc/{{ id }}"
# OooOOooOO templating
method = "{% if delete %}DELETE{% else %}GET{% endif %}"
help = "View a document"

[[arg]]
  name = "id"
  required = true

[[arg]]
  # specifies the --long-flag, note the omitted `--`
  long = "delete"
  short = 'd'
  name = "delete"
  # we default to a string, but this is a bool
  type = "bool"
  # when the flag is present we want the value to be true
  action = "set_true"
```

Many values within the template are... well, templated... hence the name. They use the context from `[[arg]]` to help define things like uri, method, body, etc. The templating engine is [Tera](https://keats.github.io/tera/docs/). Now we can run the following

```bash
kla run doc 233
kla run doc 233 --delete
```

Next lets address the body

```toml
uri = "/api/v1/doc/{{ id }}"
method = "{% if delete %}DELETE{% else %}GET{% endif %}"
help = "View a document"
body = """
{"force": {{ force }} }
"""

[[arg]]
  name = "id"
  required = true

[[arg]]
  long = "delete"
  short = 'd'
  name = "delete"
  type = "bool"
  action = "set_true"

[[arg]]
  long = "force"
  name = "force"
  type = "bool"
  # notice the default is a string!
  default = "false"
  action = "set_true"
```

Whew looks like we are done!

```bash
kla run doc 233
kla run doc 233 --delete --force
```

Hold up though! Melisa from the docs department API department just let you know you can specify a `format` as a query param where you can select `json` or `yaml`. You remind Melisa that there is an HTTP header better suited for that, but she rolls her eyes at you; Typical Melisa! We could use templating in the uri, something like `uri = "/api/v1/doc/{{ id }}?format={{ format }}"` but Melisa has informed you they haven't landed on the final format yet so you want your template to utilize the APIs default value instead of hardcoding your own.


```toml
  ...

  default = "false"
  action = "set_true"

# The new stuff!
[[arg]]
  long = "format"
  name = "format"

[[query]]
  name = "format"
  value = "format"
  when = "{% if format %}yes{% endif %}"
```

Now when you supply a format to the command it will render as a query parameter.

```
kla run doc 233 --format yaml
```

The `when` operator will add the query parameter when it has a non empty value.. so `yes` could have also been `four score and blah blah blah`.

# Configuration

Alright, here is the full configuration available to you now that you are familiarized with things.

```toml
# The short description is used to define the short help, and will be displayed
# when you run `kla run --help`
short_description = "Example template, helping define how to make templates"

# The long description which is shown when you run `kla run [template] --help`
# It's often a good idea to link to any documentation on the endpoint you are
# hitting here
description = """
A Very long description, though this one isn't
"""

# An argument creates a new `--flag` or `<argument>` that can be used as "Context"
# This "Context" are key value pairs which can be used within many locations of the
# template.
#
# [[arg]] is an array that we are adding to. For this first one I will specify a
# real example, the next one will have all the values.
[[arg]]
  help = "A message that you want to send to the API"
  name = "message"
  required = true

# all values
[[arg]]
  # The name of the argument, use this when referencing it's value inside
  # templates
  name = "varname"

  # Type specifies the type of value we expect, valid values are
  # - string (default when not present)
  # - number
  # - bool
  type = "string"

  # Boolean specifying if this is a list of arguments, or a single argument
  many_valued = false

  # short specifies the shorthand flag. If this and `long` are not specified
  # the arg is considered a regular argument instead of a flagged argument
  # -c
  short = 'c'

  # short_aliases allows you to define additional shorthand flags for this
  # argument
  short_aliases = ['a', 'b', 'c']

  # long specifies the `--long-flag` form of the argument. If this and short
  # are not specified it assumes the arg is a normal argument.
  long = "my-long-flag"

  # Aliases specifies aliases for the `--long-form` flags
  aliases = ["another-long-flag", "what"]

  # help text which is displayed when running `kla run <template> --help`
  help = "the help text for the argument"

  # long help text which is displayed when running `kla run <template> --help`
  long_help = "the help text for the argument"

  # next_line_help allows you to specify additional text after the help text
  # has rendered
  next_line_help = """
This is some more text that I really want rendered at the end of the help
text
"""

  # required defines if the argument is required or not
  required = true

  # trailing_var_arg lets you capture all the values into an array
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.trailing_var_arg
  trailing_var_arg = false

  # last specifies that this should be the very last argument
  last = false

  # exclusive specifies this should be the only argument present
  exclusive = false

  # value_name Placeholder for the argument’s value in the help message / usage.
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.value_name
  value_name = "something"

  # allow_hyphen_values enables the argument to consume what look like flagged
  # values
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.allow_hyphen_values
  allow_hyphen_values = false

  # allow_negative_numbers allows you to consume negative numbers as arguments
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.allow_negative_numbers
  allow_negative_numbers = false

  # require_equals forces the user to use the `--long-flag=value` syntax
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.require_equals
  require_equals = false

  # value_delimiter Allow grouping of multiple values via a delimiter. Must
  # set attribute `many_valued = true`
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.value_delimiter
  value_delimiter = ','

  # value_terminator Sentinel to stop parsing multiple values of a given argument.
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.value_terminal
  value_terminal = ';'

  # Consume all following arguments.
  # 
  # Do not parse them individually, but rather pass them in entirety.
  # 
  # It is worth noting that setting this requires all values to come after
  # a -- to indicate they should all be captured. For example:
  # 
  # --foo something -- -v -v -v -b -b -b --baz -q -u -x
  # 
  # Will result in everything after -- to be considered one raw argument.
  # This behavior may not be exactly what you are expecting and using
  # trailing_var_arg may be more appropriate.
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.raw
  raw = false

  # The string representation of your default value. For Numbers and
  # Bools you should specify the value as a string
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.default_value
  default_value = "something"

  # The string representation of your values when `many_valued = true`
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.default_values
  default_values = ["something", "Another Thing"]

  # When a flag is present, but no value was given, you can specify the
  # default value with `default_missing_value`
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.default_missing_value
  default_missing_value = "something"

  # When a flag is present, but no value was given, you can specify the
  # default values with `default_missing_values`, assuming `many_valued = true`
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.default_missing_values
  default_missing_values = ["something", "Another Thing"]

  # env specifies the environment variable this value should inherit
  # when not present
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.env
  env = "KLA_HTTP_BASIC_AUTH"

  # There are a collection of hide attributes which attempt to not disclose
  # sensative information in the help text.

  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.hide_possible_values
  hide_possible_values = false
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.hide_default_value
  hide_default_value = false
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.hide_env
  hide_env = false
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.hide_env_values
  hide_env_values = false
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.hide_short_help
  hide_short_help = false
  # https://docs.rs/clap/latest/clap/struct.Arg.html#method.hide_long_help
  hide_long_help = false

  # action defines what should happen when an argument is parsed. Most of
  # the time you weant to set a value, but there are other things you can do
  # - set: (default) set the argument to a value
  # - append: Append to the current argument
  # - set_true: when `type = bool` this sets the value to true
  # - set_false: when `type = bool` this sets the value to false
  # - count: Increment a counter as the value
  # - help: output the help command
  # - help_short: output the short help command
  # - help_long: output the long help command
  # - version: output the version of the application
  action = 'set'

  # password will show a password prompt if the value is empty, so you
  # don't have to pass that in
  password = false

# Body is a template that uses values constructed from [[arg]] to create the
# http body. We use Tera (https://keats.github.io/tera/docs/) as the templating
# engine
body = """
{"message": "I want to share a message: {{ message }}"}
"""

# uri is the endpoint location, the environment url will be prepended.
# This field is templated with Tera (https://keats.github.io/tera/docs/)
uri = "/admin/message"

# method is the http request method. GET, POST, PUT, HEAD, DELETE
# This field is templated with Tera (https://keats.github.io/tera/docs/)
method = "GET"

# Header enables you to specify an http header in the request
[[header]]
  # The header name is not templated.
  name = "x-my-message"
  # value specifies the value of the header
  # This field is templated with Tera (https://keats.github.io/tera/docs/)
  value = "{{ message }}"
  # When specifies when the header *should* be added as a header. If this value
  # renders to a non empty string the header will be added.
  when = "{{ message | default(value="") }}"


# query enables you to specify an http query parameter
[[query]]
  # The query parameter name is not templated.
  name = "message"
  # value specifies the value of the query parameter
  # This field is templated with Tera (https://keats.github.io/tera/docs/)
  value = "{{ message }}"
  # When specifies when the query parameter *should* be added. If this value
  # renders to a non empty string the query parameter will be added.
  when = "{{ message | default(value="") }}"


# form enables you to specify an http form parameter
[[form]]
  # The form parameter name is not templated.
  name = "message"
  # value specifies the value of the form parameter
  # This field is templated with Tera (https://keats.github.io/tera/docs/)
  value = "{{ message }}"
  # When specifies when the form parameter *should* be added. If this value
  # renders to a non empty string the form parameter will be added.
  when = "{{ message | default(value="") }}"

# Once the http request has been sent and we get a response we can template
# the output. By default the response is just written out.
# Kla attempts to deserialized the response body and the corresponding values
# are added to the Context.
# Additional values available are:
# - resp_status: The response status as a string
# - resp_headers_{}: Each response header
# - resp_http_version: The http version
# - resp_body: The raw http response body
# 
# this example assumes a response body of `{"recipient": "Terry Cruze"}`
template = """
The server responded with {{ resp_status }} and a value {{ recipient }}
"""

# Things don't always go well. You can specify a template to render when the
# http response code isn't a successful one. It has all the same behavior
# of `template`
# this example assumes a response body of `{"error": "Terry Cruze doesn't want your messages"}`
template_failure = """
The server failed with the following message:
{{ error }}
"""

# output specifies where you would like the output of this template to go
# usually you want it to go to stdout, which it does by default, but for
# login endpoints maybe you want to direct things towards a file?
# this value supports templating
output = "~/.cache/my_token"

# Additionally, you might want to redirect the failure output somewhere
# else, especially if you don't want it to go to where you specified `output`
# `-` means standard out
# this value supports templating
failure_output = "-"

# Settings allow you to specify request level settings for the template.
[settings]
# allows you to specify the bearer token header. based on the first character
# this settings can read from a file
# `-` => Read from standard input
# `@` => Read from file (followed by a path `@/tmp/myfile.txt`), if the file doesn't
#        exist we do not set the value
# `!` => Read from file (followed by a path `!/tmp/myfile.txt`), if the files doesn't
#        exists return an error
# .*  => Read anything else in as the literal value
bearer_token = ""

# set the basic auth header. This has the same feature set as bearer_token
basic_auth = ""

# Timeout, configuration defined by https://docs.rs/duration-string/latest/duration_string/index.html
timeout = "10s"

# Set the HTTP Version, allowed versions are: 0.9, 1.0, 1.1, 2.0, 3.0.
http_version = "1.0"
```
