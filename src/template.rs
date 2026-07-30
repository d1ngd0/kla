use std::ffi::OsString;
use std::iter;
use std::str::from_utf8;

use anyhow::Context as _;
use clap::{arg, ArgAction, ArgMatches, Command};
use log::{debug, info};
use reqwest::{RequestBuilder, Response};
use tera::{Context, Tera};

use crate::clap::ArgOptions;
use crate::config::{ConfigCommand, FilterWhen as _};
use crate::{
    AsyncResult as _, Attributes, Environment, Error, FetchMany as _, KlaRequest as _,
    KlaRequestBuilder, Opt, Output, OutputBuilder, Result, Sigv4Request, When as _,
    WithAttributes as _,
};

#[derive(Clone, Debug, Default)]
/// Template Builder is used to create a new template. Required fields are
/// - config, set through `Self::config` or `Self::try_config`
/// - client, set through `Self::client`
/// Everything else is optional.
pub struct TemplateBuilder {
    /// config specifies the configCommand for this template.
    config: Option<ConfigCommand>,
    /// Optional context that serves as the base context we will render out of
    /// arguments.
    context: Option<Context>,
}

impl TemplateBuilder {
    /// New Creates a new template builder. It just calls `default`
    /// which returns an empty builder. You are still required to add
    /// - ConfigCommand
    /// - Client
    /// before calling `build`
    pub fn new() -> Self {
        Self::default()
    }

    /// config sets the configuration for the template. This field is
    /// required to call build, so please call some variation of it
    pub fn config<C: Into<ConfigCommand>>(mut self, config: C) -> Self {
        self.config = Some(config.into());
        self
    }

    /// try_config trys to sets the configuration based on the TryInto trait
    /// The error must implement Into<kla::Error>. config is required so call
    /// this or config!
    pub fn try_config<E: Into<Error>, C: TryInto<ConfigCommand, Error = E>>(
        mut self,
        config: C,
    ) -> Result<Self> {
        self.config = Some(config.try_into().map_err(E::into)?);
        Ok(self)
    }

    /// context sets the context we will build upon. This is not required, and we will
    /// call Context::default() when not provided. The context is often derived through
    /// `[[arg]]` via the template. So anything provided here is just additional sugar.
    pub fn context<A: Into<Context>>(mut self, context: A) -> Self {
        self.context = Some(context.into());
        self
    }

    /// try_context is the same as context, but uses the TryInto trait instead of Into.
    /// the Error returned in your TryInto must implement Into<kla::Error>
    pub fn try_context<E: Into<crate::Error>, A: TryInto<Context, Error = E>>(
        mut self,
        context: A,
    ) -> Result<Self> {
        self.context = Some(context.try_into().map_err(E::into)?);
        Ok(self)
    }

    /// build the template
    pub fn build(self) -> Result<Template> {
        let Self { config, context } = self;

        let config =
            config.ok_or_else(|| anyhow::Error::msg("config is required to create a template!"))?;
        let mut tmpl = Tera::default();
        tmpl.add_raw_templates(config.templates()?)
            .context("invalid template")?;

        let context = context.unwrap_or_else(|| Context::default());

        Ok(Template {
            tmpl,
            context,
            config,
        })
    }
}

#[derive(Clone, Debug)]
/// Template is a runnable template which takes an environment and a set of arguments
/// to run
pub struct Template {
    tmpl: Tera,
    context: Context,
    config: ConfigCommand,
}

impl TryFrom<&Template> for Command {
    type Error = crate::Error;

    fn try_from(value: &Template) -> std::result::Result<Self, Self::Error> {
        Command::try_from(value.config.clone())
    }
}

impl Template {
    pub async fn run_matches_from<I, T, E>(&self, env: &E, args: I, dry: bool) -> Result<Output>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
        E: Environment,
    {
        // this code is really stupid, and I hate it, we are hacking the shit out of things here
        // but I really don't care since this drives collections and collections are shit.
        // I will put some thought into this at some point but it **is not this day**
        let cmd = Command::try_from(self)?.arg(
            arg!(--dry "This is really gross, but this serves collections which are stupid")
                .action(ArgAction::SetTrue),
        );
        let args =
            iter::once(OsString::from(&self.config.name)).chain(args.into_iter().map(|s| s.into()));
        let args = if dry {
            cmd.get_matches_from(args.chain(iter::once(OsString::from("--dry"))))
        } else {
            cmd.get_matches_from(args)
        };

        self.run(env, &args, &args).await
    }

    pub async fn run<E>(&self, env: &E, args: &ArgMatches, parent: &ArgMatches) -> Result<Output>
    where
        E: Environment,
    {
        let mut context = self.context.clone();
        context.extend(
            self.config
                .args_context(args)
                .context("Invalid Arguments Supplied")?,
        );

        let context = env.context(context).with_context(|| {
            format!(
                "could not set context value from environment {}",
                env.name()
            )
        })?;

        debug!("Context for template {:#?}", &context);

        let tmpl_attrs: Attributes = self.config.attrs.as_ref().try_into()?;
        let request = env
            .request(
                self.tmpl
                    .render("method", &context)
                    .with_context(|| format!("could not render method template"))?
                    .to_uppercase()
                    .as_str(),
                &self
                    .tmpl
                    .render("uri", &context)
                    .with_context(|| format!("could not render uri template"))?,
            )?
            .with_attributes(&tmpl_attrs)?
            .with_some(
                self.tmpl
                    .render("body", &context)
                    .map(|v| Some(v))
                    .or_else(|err| match err.kind {
                        tera::ErrorKind::TemplateNotFound(_) => Ok(None),
                        _ => Err(err),
                    })
                    .with_context(|| format!("could not render body template"))?,
                RequestBuilder::body,
            )
            .opt_headers(Some(
                self.tmpl
                    .fetch_with_prefix("header.", &context)
                    .filter_map(|v| match self.config.header.filter_when(&v) {
                        Ok(true) => Some(Ok(v)),
                        Ok(false) => None,
                        Err(err) => Some(Err(err)),
                    })
                    .collect::<Result<Vec<_>>>()?
                    .into_iter(),
            ))
            .with_context(|| format!("headers could not be loaded"))?
            .opt_query(Some(
                self.tmpl
                    .fetch_with_prefix("query.", &context)
                    .filter_map(|v| match self.config.query.filter_when(&v) {
                        Ok(true) => Some(Ok(v)),
                        Ok(false) => None,
                        Err(err) => Some(Err(err)),
                    })
                    .collect::<Result<Vec<_>>>()?
                    .into_iter(),
            ))
            .with_context(|| format!("query params could not be loaded",))?
            .opt_form(Some(
                self.tmpl
                    .fetch_with_prefix("form.", &context)
                    .filter_map(|v| match self.config.form.filter_when(&v) {
                        Ok(true) => Some(Ok(v)),
                        Ok(false) => None,
                        Err(err) => Some(Err(err)),
                    })
                    .collect::<Result<Vec<_>>>()?
                    .into_iter(),
            ))
            .with_context(|| format!("form params could not be loaded",))?
            .with_arg_opts(parent)?
            .build()
            .map_err(Error::from)
            .and_then(|req| env.sign(req))
            .when(parent.get_count("verbose") > 0, |f| {
                f.map(|r| {
                    info!(
                        "{}",
                        r.body()
                            .map(|f| f.as_bytes().unwrap_or_default())
                            .map(|f| from_utf8(f).unwrap_or_default())
                            .unwrap_or_default()
                    );
                    r
                })
            })
            .async_and_then(async |req| req.edit(parent.get_one::<bool>("edit")).await)
            .await
            .context("could not build http request")?;

        let request = if args.get_one("sigv4").map(|v| *v).unwrap_or(false) {
            request
                .sign_request(
                    args.get_one::<String>("sigv4-aws-profile"),
                    args.get_one::<String>("sigv4-aws-service"),
                )
                .await?
        } else {
            request
        };
        info!("Request: {:#?}", request);

        let response = match parent.get_one("dry").copied().unwrap_or_default() {
            true => Response::from(http::Response::<Vec<u8>>::default()),
            false => env
                .execute(request)
                .await
                .with_context(|| format!("request failed!"))?,
        };
        info!("Response: {:#?}", response);

        let succeed = response.status().is_success();

        OutputBuilder::new().with_some_result(match succeed {
                true => self.config.template.as_ref(),
                false => self.config.template_failure.as_ref(),
            }, OutputBuilder::template)
        .with_context(|| format!("Your request was sent but the output or failure-template within could not be parsed, run with -v to see if your request was successful"))?
        .with_some(self.tmpl
                    .render(match succeed {
                        true => "output",
                        false => "output_failure",
                    }, &context)
                    .map(|v| Some(v))
                    .or_else(|err| match err.kind {
                        tera::ErrorKind::TemplateNotFound(_) => Ok(None),
                        _ => Err(err),
                    })
                    .with_context(|| format!("could not render body template"))?,
                OutputBuilder::desired_location)
        .with_some_result(match succeed {
            true => args.get_one::<String>("template"),
            false => args.get_one::<String>("failure-template"),
        }, OutputBuilder::template)
        .with_context(|| format!("Your request was sent but the --template or --failure-template could not be parsed, run with -v to see if your request was successful"))?
        .build(response)
        .await.with_context(|| format!("could not write output to specified location!")).map_err(crate::Error::from)
    }
}
