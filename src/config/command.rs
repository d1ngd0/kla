use std::{fs, path::Path};

use anyhow::Context as _;
use clap::{command, Arg, ArgAction, ArgMatches, Command};
use inquire::Password;
use serde::{de::Visitor, Deserialize, Deserializer};
use tera::{Context, Number, Tera};

use crate::{Ok, Opt, RenderGroup};

#[derive(Deserialize, Clone, Debug)]
pub struct ConfigCommand {
    #[serde(skip)]
    pub name: String,

    #[serde(rename = "short_description")]
    short_description: Option<String>,

    #[serde(rename = "description")]
    description: Option<String>,

    #[serde(rename = "arg", default)]
    args: Vec<ConfigArg>,

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
    #[serde(rename = "template", skip)]
    pub template: Option<String>,
    #[serde(rename = "template_failure", skip)]
    pub template_failure: Option<String>,
    #[serde(rename = "output", skip)]
    pub output: Option<String>,
    #[serde(rename = "output_failure", skip)]
    pub output_failure: Option<String>,
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
    fn filter_when(&self, tmpl: &RenderGroup<'_>) -> crate::Result<bool>;
}

impl FilterWhen for Vec<ConfigKV> {
    /// filter_when filters the when clause in the ConfigKV
    fn filter_when(&self, tmpl: &RenderGroup<'_>) -> crate::Result<bool> {
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
        Self::with_name(name, content)
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

    pub fn templates<'a>(&'a self) -> crate::Result<Vec<(String, &'a String)>> {
        let mut templates: Vec<(String, &'a String)> = vec![];

        if let Some(body) = self.body.as_ref() {
            templates.push(("body".into(), &body));
        }

        templates.push(("uri".into(), &self.uri));
        templates.push(("method".into(), &self.method));

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
    pub fn args_context(&self, args: &ArgMatches) -> crate::Result<Context> {
        macro_rules! get_one {
    ($args:expr, $ty:ty, $name:expr) => {
        $args
            .try_get_one::<$ty>($name)
            .map_err(|_| {
                crate::Error::from(format!(
                    "argument `{}` had type of `{}` which is apparently wrong, set `type` to the correct value in your template",
                    $name,
                    stringify!($ty),
                ))
            })?
    };
}

        macro_rules! get_many {
            ($args:expr, $ty:ty, $name:expr) => {
                $args
                    .try_get_many::<$ty>($name)
                    .map_err(|_| {
                        crate::Error::from(format!(
                            "{} type of {} wrong, set `type` to the correct value",
                            $name,
                            stringify!($ty),
                        ))
                    })?
                    .map(|v| v.collect::<Vec<_>>())
            };
        }

        let mut ctx = Context::new();
        for arg in &self.args {
            match arg.arg_type {
                ConfigArgType::String if arg.many_valued => {
                    get_many!(args, String, &arg.name)
                        .iter()
                        .for_each(|v| ctx.insert(&arg.name, &v));
                }
                ConfigArgType::String if arg.password => get_one!(args, String, &arg.name)
                    .map(|v| v.clone())
                    .or_else(|| {
                        Password::new("Password:")
                            .without_confirmation()
                            .prompt()
                            .ok()
                    })
                    .iter()
                    .for_each(|v| ctx.insert(&arg.name, v)),
                ConfigArgType::String => get_one!(args, String, &arg.name)
                    .iter()
                    .for_each(|v| ctx.insert(&arg.name, v)),
                ConfigArgType::Number if arg.many_valued => {
                    get_many!(args, Number, &arg.name)
                        .iter()
                        .for_each(|v| ctx.insert(&arg.name, &v));
                }
                ConfigArgType::Number => get_one!(args, Number, &arg.name)
                    .iter()
                    .for_each(|v| ctx.insert(&arg.name, v)),
                ConfigArgType::Bool if arg.many_valued => {
                    get_many!(args, bool, &arg.name)
                        .iter()
                        .for_each(|v| ctx.insert(&arg.name, &v));
                }
                ConfigArgType::Bool => get_one!(args, bool, &arg.name)
                    .iter()
                    .for_each(|v| ctx.insert(&arg.name, v)),
            }
        }

        Ok(ctx)
    }
}

impl TryFrom<ConfigCommand> for Command {
    type Error = crate::Error;

    fn try_from(value: ConfigCommand) -> Result<Self, Self::Error> {
        let command = command!()
            .name(&value.name)
            .with_some(value.short_description.as_ref(), Command::about)
            .with_some(value.description.as_ref(), Command::long_about)
            .with_ok_value(
                value
                    .args
                    .into_iter()
                    .map(|v| ConfigArg::try_into(v))
                    .collect::<Result<Vec<Arg>, Self::Error>>(),
                Command::args,
            )
            .with_context(|| format!("{} invalid command configuration", &value.name))?;

        Ok(command)
    }
}

#[derive(Deserialize, Copy, Clone, Debug)]
pub enum ConfigArgType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "bool")]
    Bool,
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
    default_value: Option<String>,
    #[serde(rename = "default_values", default)]
    default_values: Option<Vec<String>>,
    #[serde(rename = "default_missing_value")]
    default_missing_value: Option<String>,
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
    #[serde(rename = "password", default)]
    password: bool,
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

/// Implementation of turining a ConfigArg into an Argument for
/// clap.
impl TryFrom<ConfigArg> for Arg {
    type Error = crate::Error;

    fn try_from(value: ConfigArg) -> Result<Self, Self::Error> {
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
            .with_some(value.default_value, Arg::default_value)
            .with_some(value.default_values, Arg::default_values)
            .with_some(value.default_missing_value, Arg::default_missing_value)
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
