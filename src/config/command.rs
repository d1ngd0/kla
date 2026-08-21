use std::{
    fs::{self, DirEntry},
    ops::Deref,
    path::{absolute, Path, PathBuf},
    str::FromStr,
};

use clap::{command, Arg, ArgAction, ArgMatches, Command};
use parse_datetime::{parse_datetime, ParsedDateTime};
use serde::{de::Visitor, Deserialize, Deserializer};
use tera::{Context, Number, Tera};

use super::{Attributes, ValueSource};
use crate::{clap::arg_file_value, Context as _, Error, KResult, Ok, Opt, RenderGroup};

#[derive(Deserialize, Clone, Debug)]
pub struct ConfigCommand {
    #[serde(skip)]
    pub name: String,
    #[serde(skip)]
    pub subcommands: Vec<ConfigCommand>,

    #[serde(rename = "short_description")]
    short_description: Option<String>,

    #[serde(rename = "description")]
    description: Option<String>,

    #[serde(rename = "arg", default)]
    args: ConfigArgCollection,

    #[serde(rename = "body")]
    body: Option<String>,
    #[serde(rename = "uri", default = "default_uri")]
    uri: String,
    #[serde(rename = "method", default = "default_method")]
    method: String,
    #[serde(rename = "header", default)]
    pub(crate) header: Vec<ConfigKV>,
    #[serde(rename = "query", default)]
    pub(crate) query: Vec<ConfigKV>,
    #[serde(rename = "form", default)]
    pub(crate) form: Vec<ConfigKV>,

    // these are utilized by OutputBuilder
    #[serde(rename = "template")]
    pub template: Option<String>,
    #[serde(rename = "template_failure")]
    pub template_failure: Option<String>,
    #[serde(rename = "output")]
    pub output: Option<String>,
    #[serde(rename = "output_failure")]
    pub output_failure: Option<String>,
    #[serde(rename = "settings", default)]
    pub attrs: Attributes,
}

// default_uri specifies the default uri when one is not supplied
fn default_uri() -> String {
    String::from("/")
}

// default_method specified the default method when one is not supplied
fn default_method() -> String {
    String::from("GET")
}

// HeaderConfig defines the values in the config needed to create a header
#[derive(Deserialize, Debug, Clone)]
pub struct ConfigKV {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "value")]
    pub value: String,
    #[serde(rename = "when")]
    pub when: Option<String>,
}

pub trait FilterWhen {
    fn filter_when(&self, tmpl: &RenderGroup<'_>) -> crate::KResult<bool>;
}

impl FilterWhen for Vec<ConfigKV> {
    /// filter_when filters the when clause in the ConfigKV
    fn filter_when(&self, tmpl: &RenderGroup<'_>) -> crate::KResult<bool> {
        self.iter()
            .filter(|v| v.name == tmpl.name)
            .next()
            .map(|v| v.when.as_ref())
            .flatten()
            .map(|v| Tera::one_off(v, tmpl.context, true).map(|v| v != ""))
            .unwrap_or(Ok(true))
            .map_err(crate::Error::from)
    }
}

impl ConfigCommand {
    pub fn from_file<P>(path: P) -> std::result::Result<ConfigCommand, crate::Error>
    where
        P: AsRef<Path>,
    {
        let dir = absolute(path.as_ref())?;
        let dir = dir
            .parent()
            .with_context(|| format!("issue reading absolute path for {:?}", path.as_ref()))?;

        let name = path
            .as_ref()
            .file_name()
            .and_then(|filename| filename.to_str())
            .and_then(|filename| filename.strip_suffix(".toml"))
            .ok_or_else(|| {
                crate::Error::from(format!(
                    "could not get command name from path {:?}",
                    path.as_ref()
                ))
            })?;
        let content = fs::read_to_string(path.as_ref())?;
        let mut config = Self::with_name(name, content)?;
        config.resolve_working_dir(dir);

        // create the directory name for subcommands
        // and set the value to `Some` if the directory
        // exists
        let subcmd_dir = path
            .as_ref()
            .parent()
            .map(PathBuf::from)
            .map(|mut v| {
                v.push(format!("{}.subcmd", name));
                v
            })
            .filter(|path| path.is_dir());

        if let Some(subcmd_dir) = subcmd_dir {
            config.with_subcommands(subcmd_dir)
        } else {
            Ok(config)
        }
    }

    /// resolve_working_dir will go through all the fields that are paths
    /// and resolve them to the provided working dir if they are relative
    pub fn resolve_working_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.attrs.resolve_working_dir(dir);
    }

    fn with_subcommands<P: AsRef<Path>>(self, path: P) -> Result<Self, crate::Error> {
        let mut config = self;
        config.subcommands = fs::read_dir(path.as_ref())
            .with_context(|| format!("could not read template directory {:?}", path.as_ref()))?
            .collect::<std::result::Result<Vec<DirEntry>, std::io::Error>>()?
            .into_iter()
            .filter(|f| f.file_type().map(|v| v.is_file()).unwrap_or(false))
            .map(|v| ConfigCommand::from_file(v.path()))
            .collect::<crate::KResult<Vec<ConfigCommand>>>()?;

        Ok(config)
    }

    pub fn with_name<S, C>(name: S, conf: C) -> std::result::Result<ConfigCommand, crate::Error>
    where
        S: Into<String>,
        C: AsRef<str>,
    {
        let mut conf: ConfigCommand = toml::from_str(conf.as_ref())?;
        conf.name = name.into();
        Ok(conf)
    }

    pub fn templates<'a>(&'a self) -> crate::KResult<Vec<(String, &'a String)>> {
        let mut templates: Vec<(String, &'a String)> = vec![];

        if let Some(body) = self.body.as_ref() {
            templates.push(("body".into(), &body));
        }

        templates.push(("uri".into(), &self.uri));
        templates.push(("method".into(), &self.method));

        if let Some(output) = &self.output {
            templates.push(("output".into(), output));
        }

        for header in &self.header {
            templates.push((format!("header.{}", header.name), &header.value));
        }

        for query in &self.query {
            templates.push((format!("query.{}", query.name), &query.value));
        }

        for form in &self.form {
            templates.push((format!("form.{}", form.name), &form.value));
        }

        Ok(templates)
    }

    // args_context returns a Tera Context object from the arguments specifified
    pub fn args_context(&self, args: &ArgMatches) -> crate::KResult<Context> {
        self.args.args_context(args)
    }
}

impl TryFrom<ConfigCommand> for Command {
    type Error = crate::Error;

    fn try_from(value: ConfigCommand) -> Result<Self, Self::Error> {
        let mut command = command!()
            .name(&value.name)
            .with_some(value.short_description.as_ref(), Command::about)
            .with_some(value.description.as_ref(), Command::long_about)
            .with_ok_value(<Vec<Arg>>::try_from(value.args), Command::args)
            .with_context(|| format!("{} invalid command configuration", &value.name))?;

        // Add the subcommands
        for config in value.subcommands {
            let subcommand = Command::try_from(config)
                .with_context(|| format!("subcommand of {}", &value.name))?;
            command = command.subcommand(subcommand);
        }

        Ok(command)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub enum ConfigArgType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "datetime")]
    Datetime(String),
}

impl AsRef<Self> for ConfigArgType {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl Default for ConfigArgType {
    fn default() -> Self {
        Self::String
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ConfigArg {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "type", default)]
    arg_type: ConfigArgType,
    #[serde(rename = "many_valued", default)]
    many_valued: bool,
    #[serde(rename = "short")]
    short: Option<char>,
    #[serde(rename = "short_aliases", default)]
    short_aliases: Vec<char>,
    #[serde(rename = "long")]
    long: Option<String>,
    #[serde(rename = "aliases", default)]
    aliases: Vec<String>,
    #[serde(rename = "help")]
    help: Option<String>,
    #[serde(rename = "long_help")]
    long_help: Option<String>,
    #[serde(rename = "next_line_help")]
    next_line_help: Option<bool>,
    #[serde(rename = "required")]
    required: Option<bool>,
    #[serde(rename = "trailing_var_arg")]
    trailing_var_arg: Option<bool>,
    #[serde(rename = "last")]
    last: Option<bool>,
    #[serde(rename = "exclusive")]
    exclusive: Option<bool>,
    #[serde(rename = "value_name")]
    value_name: Option<String>,
    #[serde(rename = "allow_hyphen_values")]
    allow_hyphen_values: Option<bool>,
    #[serde(rename = "allow_negative_numbers")]
    allow_negative_numbers: Option<bool>,
    #[serde(rename = "require_equals")]
    require_equals: Option<bool>,
    #[serde(rename = "value_delimiter")]
    value_delimiter: Option<char>,
    #[serde(rename = "value_terminator")]
    value_terminator: Option<String>,
    #[serde(rename = "raw")]
    raw: Option<bool>,
    #[serde(rename = "default_value")]
    default_value: Option<ValueSource>,
    #[serde(rename = "default_values", default)]
    default_values: Option<Vec<String>>,
    #[serde(rename = "default_missing_value")]
    default_missing_value: Option<ValueSource>,
    #[serde(rename = "default_missing_values", default)]
    default_missing_values: Option<Vec<String>>,
    #[serde(rename = "env")]
    env: Option<String>,
    #[serde(rename = "hide")]
    hide: Option<bool>,
    #[serde(rename = "hide_possible_values")]
    hide_possible_values: Option<bool>,
    #[serde(rename = "hide_default_value")]
    hide_default_value: Option<bool>,
    #[serde(rename = "hide_env")]
    hide_env: Option<bool>,
    #[serde(rename = "hide_env_values")]
    hide_env_values: Option<bool>,
    #[serde(rename = "hide_short_help")]
    hide_short_help: Option<bool>,
    #[serde(rename = "hide_long_help")]
    hide_long_help: Option<bool>,
    #[serde(
        rename = "action",
        deserialize_with = "deserialize_action",
        default = "arg_action_default"
    )]
    action: Option<ArgAction>,
    #[serde(rename = "file_value", default)]
    file_value: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConfigArgCollection(Vec<ConfigArg>);

impl Deref for ConfigArgCollection {
    type Target = Vec<ConfigArg>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ConfigArgCollection {
    // args_context returns a Tera Context object from the arguments specifified
    pub fn args_context(&self, args: &ArgMatches) -> crate::KResult<Context> {
        macro_rules! get_one {
            ($args:expr, $ty:ty, $arg:expr) => {
                    match $args
                    .try_get_one::<$ty>(&$arg.name)
                    .map_err(|err| Error::invalid_arguments(err))
                    .with_context(|| format!(
                            "argument `{}` had type of `{}` which is apparently wrong, set `type` to the correct value in your template",
                            &$arg.name,
                            stringify!($ty),
                    ))? {
                        Some(v) => Some(v.clone()),
                        None => {
                            if args.contains_id(&$arg.name) {
                                $arg.default_missing_value
                                    .as_ref()
                                    .map(|v| v.clone().to_string())
                                    .transpose()?
                                    .map(<$ty as ValueSourceParser>::parse)
                                    .transpose()?
                            } else {
                                $arg.default_value
                                    .as_ref()
                                    .map(|v| v.clone().to_string())
                                    .transpose()?
                                    .map(<$ty as ValueSourceParser>::parse)
                                    .transpose()?
                            }
                        }
                    }

            };
        }

        macro_rules! get_many {
            ($args:expr, $ty:ty, $name:expr) => {
                $args
                    .try_get_many::<$ty>($name)
                    .map_err(|err| Error::invalid_arguments(err))
                    .with_context(|| {
                        format!(
                            "{} type of {} wrong, set `type` to the correct value",
                            $name,
                            stringify!($ty),
                        )
                    })?
            };
        }

        let mut ctx = Context::new();
        for arg in self.iter() {
            match arg.arg_type.as_ref() {
                // Many Valued String
                ConfigArgType::String if arg.many_valued => {
                    get_many!(args, String, &arg.name)
                        .unwrap_or_default()
                        .for_each(|v| ctx.insert(&arg.name, v));
                }

                // File Value String
                ConfigArgType::String if arg.file_value => {
                    arg_file_value(get_one!(args, String, arg).as_ref(), &arg.name)?
                        .inspect(|v| ctx.insert(&arg.name, v));
                }

                // Just a String
                ConfigArgType::String => {
                    get_one!(args, String, arg).inspect(|v| ctx.insert(&arg.name, v));
                }

                // Many Valued Number
                ConfigArgType::Number if arg.many_valued => {
                    get_many!(args, Number, &arg.name)
                        .unwrap_or_default()
                        .for_each(|v| ctx.insert(&arg.name, v));
                }

                // Just A number
                ConfigArgType::Number => get_one!(args, Number, arg)
                    .iter()
                    .for_each(|v| ctx.insert(&arg.name, v)),

                // Many Valued Boolean
                ConfigArgType::Bool if arg.many_valued => {
                    get_many!(args, bool, &arg.name)
                        .unwrap_or_default()
                        .for_each(|v| ctx.insert(&arg.name, &v));
                }

                // Just a Boolean
                ConfigArgType::Bool => {
                    get_one!(args, bool, arg).inspect(|v| ctx.insert(&arg.name, v));
                }
                ConfigArgType::Datetime(format) if arg.many_valued => {
                    let dates = get_many!(args, String, &arg.name)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|f| parse_datetime(f).map_err(Error::invalid_arguments))
                        .map(|date| {
                            date.map(|date| match date {
                                ParsedDateTime::InRange(zoned) => Ok(zoned),
                                ParsedDateTime::Extended(extended_date_time) => {
                                    Err(Error::invalid_arguments(format!(
                                        "date {} is out of range",
                                        extended_date_time
                                    )))
                                }
                            })
                        })
                        .map(|v| v.flatten())
                        .map(|f| f.map(|f| f.strftime(&format).to_string()))
                        .collect::<KResult<Vec<String>>>()?;
                    dates.iter().for_each(|v| ctx.insert(&arg.name, &v));
                }
                ConfigArgType::Datetime(format) => {
                    get_one!(args, String, arg)
                        .map(|f| parse_datetime(f))
                        .transpose()
                        .map_err(Error::invalid_arguments)?
                        .map(|date| match date {
                            ParsedDateTime::InRange(zoned) => Ok(zoned),
                            ParsedDateTime::Extended(extended_date_time) => {
                                Err(Error::invalid_arguments(format!(
                                    "date {} is out of range",
                                    extended_date_time
                                )))
                            }
                        })
                        .transpose()?
                        .map(|f| f.strftime(&format).to_string())
                        .inspect(|v| ctx.insert(&arg.name, v));
                }
            }
        }

        Ok(ctx)
    }
}

impl TryFrom<ConfigArgCollection> for Vec<Arg> {
    type Error = crate::Error;

    fn try_from(value: ConfigArgCollection) -> std::result::Result<Self, Self::Error> {
        let value = value.0;
        value.into_iter().map(|v| ConfigArg::try_into(v)).collect()
    }
}

/// arg_action_default sets the default value of arg actions
fn arg_action_default() -> Option<ArgAction> {
    None
}

/// deserialize_action is used to deserialize the ArgAction.
fn deserialize_action<'de, D>(de: D) -> Result<Option<ArgAction>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ActionVisitor;

    impl<'de> Visitor<'de> for ActionVisitor {
        type Value = Option<ArgAction>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "expected string with value `set`, `append`, `set_true`, `set_false`, `count`, `help`, `help_short`, `help_long`, `version`")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(ArgAction::Set))
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match v {
                "set" => Ok(Some(ArgAction::Set)),
                "append" => Ok(Some(ArgAction::Append)),
                "set_true" => Ok(Some(ArgAction::SetTrue)),
                "set_false" => Ok(Some(ArgAction::SetFalse)),
                "count" => Ok(Some(ArgAction::Count)),
                "help" => Ok(Some(ArgAction::Help)),
                "help_short" => Ok(Some(ArgAction::HelpShort)),
                "help_long" => Ok(Some(ArgAction::HelpLong)),
                "version" => Ok(Some(ArgAction::Version)),
                _ => Err(serde::de::Error::custom("unknown action type provided")),
            }
        }
    }

    let av = ActionVisitor {};
    de.deserialize_str(av)
}

/// ValueSourceParser is used to turn a value from a ValueSource into the value we actually
/// want
trait ValueSourceParser: Sized {
    type Error;

    fn parse(value: String) -> Result<Self, Self::Error>;
}

impl ValueSourceParser for String {
    type Error = Error;

    fn parse(value: String) -> Result<Self, Self::Error> {
        Ok(value)
    }
}

impl ValueSourceParser for bool {
    type Error = Error;

    fn parse(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(Error::invalid_arguments("expected 'true' or 'false'")),
        }
    }
}

impl ValueSourceParser for Number {
    type Error = Error;

    fn parse(value: String) -> Result<Self, Self::Error> {
        Number::from_str(&value).context("invalid number")
    }
}

/// Implementation of turining a ConfigArg into an Argument for
/// clap.
impl TryFrom<ConfigArg> for Arg {
    type Error = crate::Error;

    fn try_from(value: ConfigArg) -> Result<Self, Self::Error> {
        match value.arg_type {
            ConfigArgType::Number if value.file_value => {
                return Err(Error::from(
                    "Can not specify `file_value` when type is `number`",
                ))
            }
            ConfigArgType::Bool if value.file_value => {
                return Err(Error::from(
                    "Can not specify `file_value` when type is bool",
                ))
            }
            _ => (),
        }

        let arg = Arg::new(&value.name)
            .with_some(value.help, Arg::help)
            .with_some(value.long_help, Arg::long_help)
            .with_some(value.next_line_help, Arg::next_line_help)
            .with_some(value.short, Arg::short)
            .with_some(value.long, Arg::long)
            .aliases(value.aliases)
            .short_aliases(value.short_aliases)
            .with_some(value.required, Arg::required)
            .with_some(value.trailing_var_arg, Arg::trailing_var_arg)
            .with_some(value.exclusive, Arg::exclusive)
            .with_some(value.last, Arg::last)
            .with_some(value.allow_hyphen_values, Arg::allow_hyphen_values)
            .with_some(value.allow_negative_numbers, Arg::allow_negative_numbers)
            .with_some(value.require_equals, Arg::require_equals)
            .with_some(value.require_equals, Arg::require_equals)
            .with_some(value.value_delimiter, Arg::value_delimiter)
            .with_some(value.value_terminator, Arg::value_terminator)
            .with_some(value.value_name, Arg::value_name)
            .with_some(
                value.default_value,
                |arg, value_source| match value_source {
                    ValueSource::Value(v) => arg.default_value(v),
                    _ => arg,
                },
            )
            .with_some(value.default_values, Arg::default_values)
            .with_some(
                value.default_missing_value,
                |arg, value_source| match value_source {
                    ValueSource::Value(v) => {
                        arg.num_args(0..=1).require_equals(true).default_value(v)
                    }
                    _ => arg.num_args(0..=1).require_equals(true),
                },
            )
            .with_some(value.default_missing_values, Arg::default_missing_values)
            .with_some(value.env, Arg::env)
            .with_some(value.hide, Arg::hide)
            .with_some(value.hide_possible_values, Arg::hide_possible_values)
            .with_some(value.hide_default_value, Arg::hide_default_value)
            .with_some(value.hide_env, Arg::hide_env)
            .with_some(value.hide_env_values, Arg::hide_env_values)
            .with_some(value.hide_short_help, Arg::hide_short_help)
            .with_some(value.hide_long_help, Arg::hide_long_help)
            .with_some(value.action, Arg::action)
            .with_some(value.raw, Arg::raw);
        // at group

        Ok(arg)
    }
}
