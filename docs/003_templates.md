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

## Configuration

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

# [[arg]] is an array that we are adding to. This adds a `message` argument, which will be the first argument to the command since it is the first one specified.
# Where this argument to specify a `short` or `long` value it would be passed in as a flag instead.
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
  # - datetime
  type = "string"

  # Boolean specifying if this is a list of arguments, or a single argument. If true the same flag can be sent multiple times, and the resulting value passed into the body would be an array of values
  many_valued = false

  # short specifies the shorthand flag without the preceding -. If this and `long` are not specified
  # the arg is considered a regular argument instead of a flagged argument
  # -c
  short = 'c'

  # short_aliases allows you to define additional shorthand flags for this
  # argument
  short_aliases = ['a', 'b', 'c']

  # long specifies the `--long-flag` form of the argument without the preceding --. If this and short are not specified it assumes the arg is a normal argument.
  long = "my-long-flag"

  # Aliases specifies aliases for the `--long-form` flags
  aliases = ["another-long-flag", "what"]

  # help text which is displayed when running `kla run <template> -h`
  help = "the help text for the argument"

  # long help text which is displayed when running `kla run <template> --help`
  long_help = "the help text for the argument"

  # next_line_help allows you to specify additional text after the help text. This can be useful when there is additional context that must be considered with the flag.
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
  env = "MY_ENVIRONMENT_VARIABLE"

  # There are a collection of hide attributes which attempt to not disclose
  # sensitive information in the help text.

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
  # don't have to pass that in. This _can_ be used with `file_value` but
  # keep in mind, if your file starts with a special character there is
  # no way to escape the value.
  password = false

  # Specifies if the argument should allow the special file value prefixes
  # The argument checks for a prefix which defines how we treat the incoming value
  # `-` => Read from standard input
  # `?` => Read from file (followed by a path `?/tmp/myfile.txt`), if the file doesn't
  #        exist we return None
  # `@` => Read from file (followed by a path `@/tmp/myfile.txt`), if the files doesn't
  #        exists return an error
  file_value = true

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
# usually you want it to go to stdout, which it does by default, but maybe you have a reason to store the output to a specific file.
output = "~/.cache/my_token"

# Additionally, you might want to redirect the failure output somewhere
# else, especially if you don't want it to go to where you specified `output`
# `-` means standard out
# this value supports templating
failure_output = "-"

# Settings allow you to specify request level settings for the template. These settings are the same as specified in [environments](000_environments.md).
# [settings]
```

## Sub Commands

Each template allows for subcommands, these commands are stored in a subdirectory where the template is with `.subcmd` appended to the end of the name. For instance take the following directory:

```
/tmpls
├ config.toml
└ config.subcmd
  ├ set.toml
  └ get.toml
```

The `config.toml` is the parent command, which will load in all the subcommands from `config.subcmd`. Subcommands are nesting, meaning each subcommand can have subcommands of it's own.

## Argument Types

### String

The default type when not specified.

**Supports Multi Valued**
**Supports Password**

_example_

```toml
[[arg]]
name = "name"
# type does not need to be specified as it is the default
```

### Number

**Supports Multi Valued**

_example_

```toml
[[arg]]
name = "age"
type = "number"
```

### Boolean

**Supports Multi Valued**

_example_

```toml
[[arg]]
name = "active"
type = "bool"
```

### Datetime

**Supports Multi Valued**

Enables "Natural Language" timestamps which are then converted into the specified format. The input for the argument supports the following formats:

- num unit (e.g., "-1 hour", "+3 days")
- unit (e.g., "hour", "day")
- "now" or "today"
- "yesterday"
- "tomorrow"
- use "ago" for the past
- use "next" or "last" with unit (e.g., "next week", "last year")
- unix timestamps (for example "@0" "@1344000")

num can be a positive or negative integer. unit can be one of the following: "fortnight", "week", "day", "hour", "minute", "min", "second", "sec" and their plural forms.

_example_

```toml
[[arg]]
name = "start"
type = { datetime = "%F %T" }
```

The format string uses a “printf”-style API where conversion specifiers can be used as place holders to format components of a datetime.

This table lists the complete set of conversion specifiers supported in the format. While most conversion specifiers are supported as is in both parsing and formatting, there are some differences. Where differences occur, they are noted in the table below.

When parsing, and whenever a conversion specifier matches an enumeration of strings, the strings are matched without regard to ASCII case.

| Specifier  | Example                   | Description                                                                   |
| ---------- | ------------------------- | ----------------------------------------------------------------------------- |
| %%         | %%                        | A literal %.                                                                  |
| %A, %a     | Sunday, Sun               | The full and abbreviated weekday, respectively.                               |
| %B, %b, %h | June, Jun, Jun            | The full and abbreviated month name, respectively.                            |
| %C         | 20                        | The century of the year. No padding.                                          |
| %c         | 2024 M07 14, Sun 17:31:59 | The date and clock time via Custom. Supported when formatting only.           |
| %D         | 7/14/24                   | Equivalent to %m/%d/%y.                                                       |
| %d, %e     | 25, 5                     | The day of the month. %d is zero-padded, %e is space padded.                  |
| %F         | 2024-07-14                | Equivalent to %Y-%m-%d.                                                       |
| %f         | 000456                    | Fractional seconds, up to nanosecond precision.                               |
| %.f        | .000456                   | Optional fractional seconds, with dot, up to nanosecond precision.            |
| %G         | 2024                      | An ISO 8601 week-based year. Zero padded to 4 digits.                         |
| %g         | 24                        | A two-digit ISO 8601 week-based year. Represents only 1969-2068. Zero padded. |
| %H         | 23                        | The hour in a 24 hour clock. Zero padded.                                     |
| %I         | 11                        | The hour in a 12 hour clock. Zero padded.                                     |
| %j         | 060                       | The day of the year. Range is 1..=366. Zero padded to 3 digits.               |
| %k         | 15                        | The hour in a 24 hour clock. Space padded.                                    |
| %l         | 3                         | The hour in a 12 hour clock. Space padded.                                    |
| %M         | 04                        | The minute. Zero padded.                                                      |
| %m         | 01                        | The month. Zero padded.                                                       |
| %N         | 123456000                 | Fractional seconds, up to nanosecond precision. Alias for %9f.                |
| %n         | \n                        | Formats as a newline character. Parses arbitrary whitespace.                  |
| %P         | am                        | Whether the time is in the AM or PM, lowercase.                               |
| %p         | PM                        | Whether the time is in the AM or PM, uppercase.                               |
| %Q         | America/New_York, +0530   | An IANA time zone identifier, or %z if one doesn’t exist.                     |
| %:Q        | America/New_York, +05:30  | An IANA time zone identifier, or %:z if one doesn’t exist.                    |
| %q         | 4                         | The quarter of the year. Supported when formatting only.                      |
| %R         | 23:30                     | Equivalent to %H:%M.                                                          |
| %r         | 8:30:00 AM                | The 12-hour clock time via Custom. Supported when formatting only.            |
| %S         | 59                        | The second. Zero padded.                                                      |
| %s         | 1737396540                | A Unix timestamp, in seconds.                                                 |
| %T         | 23:30:59                  | Equivalent to %H:%M:%S.                                                       |
| %t         | \t                        | Formats as a tab character. Parses arbitrary whitespace.                      |
| %U         | 03                        | Week number. Week 1 is the first week starting with a Sunday. Zero padded.    |
| %u         | 7                         | The day of the week beginning with Monday at 1.                               |
| %V         | 05                        | Week number in the ISO 8601 week-based calendar. Zero padded.                 |
| %W         | 03                        | Week number. Week 1 is the first week starting with a Monday. Zero padded.    |
| %w         | 0                         | The day of the week beginning with Sunday at 0.                               |
| %X         | 17:31:59                  | The clock time via Custom. Supported when formatting only.                    |
| %x         | 2024 M07 14               | The date via Custom. Supported when formatting only.                          |
| %Y         | 2024                      | A full year, including century. Zero padded to 4 digits.                      |
| %y         | 24                        | A two-digit year. Represents only 1969-2068. Zero padded.                     |
| %Z         | EDT                       | A time zone abbreviation. Supported when formatting only.                     |
| %z         | +0530                     | A time zone offset in the format [+-]HHMM[SS].                                |
| %:z        | +05:30                    | A time zone offset in the format [+-]HH:MM[:SS].                              |
| %::z       | +05:30:00                 | A time zone offset in the format [+-]HH:MM:SS.                                |
| %:::z      | -04, +05:30               | A time zone offset in the format [+-]HH:[MM[:SS]].                            |

When formatting, the following flags can be inserted immediately after the % and before the directive:

    _ - Pad a numeric result to the left with spaces.
    - - Do not pad a numeric result.
    0 - Pad a numeric result to the left with zeros.
    ^ - Use alphabetic uppercase for all relevant strings.
    # - Swap the case of the result string. This is typically only useful with %p or %Z, since they are the only conversion specifiers that emit strings entirely in uppercase by default.

The above flags override the “default” settings of a specifier. For example, %_d pads with spaces instead of zeros, and %0e pads with zeros instead of spaces. The exceptions are the locale (%c, %r, %X, %x), and time zone (%z, %:z) specifiers. They are unaffected by any flags.

Moreover, any number of decimal digits can be inserted after the (possibly absent) flag and before the directive, so long as the parsed number is less than 256. The number formed by these digits will correspond to the minimum amount of padding (to the left). Note that padding is clamped to a maximum of 20.

The flags and padding amount above may be used when parsing as well. Most settings are ignored during parsing except for padding. For example, if one wanted to parse 003 as the day 3, then one should use %03d. Otherwise, by default, %d will only try to consume at most 2 digits.

The %f and %.f flags also support specifying the precision, up to nanoseconds. For example, %3f and %.3f will both always print a fractional second component to exactly 3 decimal places. When no precision is specified, then %f will always emit at least one digit, even if it’s zero. But %.f will emit the empty string when the fractional component is zero. Otherwise, it will include the leading .. For parsing, %f does not include the leading dot, but %.f does. Note that all of the options above are still parsed for %f and %.f, but they are all no-ops (except for the padding for %f, which is instead interpreted as a precision setting). When using a precision setting, truncation is used.
