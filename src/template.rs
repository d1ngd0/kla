use anyhow::Context as _;
use clap::ArgMatches;
use log::info;
use reqwest::{RequestBuilder, Response};
use tera::{Context, Tera};

use crate::clap::arg_file_value;
use crate::config::{ConfigCommand, FilterWhen as _};
use crate::{
    Environment, Error, FetchMany as _, KlaRequestBuilder, Opt, OutputBuilder, Result, Sigv4Request,
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

impl Template {
    pub async fn run<E>(&self, env: &E, args: &ArgMatches) -> Result<()>
    where
        E: Environment,
    {
        let mut context = self.context.clone();
        context.extend(
            self.config
                .args_context(args)
                .context("Invalid Arguments Supplied")?,
        );

        // TODO: Think through these, they should be applied in the following order
        // - Environment specific configuration
        // - Template specific configuration
        // - argMatch specific configuration
        // Environnment and Template should be hidden behind a single implementation
        // see `with_environment` trait, do the same for template
        // Only arg level should be specified here.
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
            .opt_timeout(self.config.timeout.as_ref())?
            .opt_version(self.config.http_version.as_ref())?
            .with_some(
                arg_file_value(self.config.bearer_token.as_ref(), "bearer_token")?,
                RequestBuilder::bearer_auth,
            )
            .with_some(
                arg_file_value(self.config.basic_auth.as_ref(), "basic_auth")?,
                |b, basic_auth| {
                    let mut parts = basic_auth.splitn(2, ":");
                    b.basic_auth(parts.next().unwrap(), parts.next())
                },
            )
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
            .opt_headers(args.get_many("header"))
            .with_context(|| {
                format!(
                    "could not set header: {:?}",
                    args.get_many::<String>("header")
                )
            })?
            // TODO: Fix `when`. Now that we are defering to render templates until we
            // actually call them we need to implement `when` here. Good call on RenderGroups
            // previous paul, they are needed now.
            // Implementation should add a filter which could be called with
            // .filter(config.filterWhen)
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
            .with_some(
                arg_file_value(args.get_one("bearer-token"), "bearer-token")?,
                RequestBuilder::bearer_auth,
            )
            .with_some(
                arg_file_value(args.get_one("basic-auth"), "basic-auth")?,
                |b, basic_auth| {
                    let mut parts = basic_auth.splitn(2, ":");
                    b.basic_auth(parts.next().unwrap(), parts.next())
                },
            )
            .opt_query(args.get_many("query"))
            .with_context(|| {
                format!(
                    "could not set query param: {:?}",
                    args.get_many::<String>("query")
                )
            })?
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
            .opt_form(args.get_many("form"))
            .with_context(|| format!("could not set form: {:?}", args.get_many::<String>("form")))?
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
            .opt_timeout(args.get_one("timeout"))
            .with_context(|| {
                format!(
                    "{:?} is not a valid format",
                    args.get_one::<String>("timeout")
                )
            })?
            .opt_version(args.get_one("http-version"))
            .with_context(|| {
                format!(
                    "{:?} is not a valid http-version",
                    args.get_one::<String>("http-version")
                )
            })?
            .build()
            .context("could not build http request")
            .and_then(|req| Ok(env.sign(req)?))?;

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

        // TODO, change verbose to log levels
        let output = OutputBuilder::new();

        // TODO: dry doesn't work
        let response = match args.get_one("dry").map(|b| *b).unwrap_or_default() {
            true => Response::from(http::Response::<Vec<u8>>::default()),
            false => env
                .execute(request)
                .await
                .with_context(|| format!("request failed!"))?,
        };
        info!("Response: {:#?}", response);

        let succeed = response.status().is_success();

        // TODO: This is shitty, and should be derived some other way. There should be
        // an output type that is generated by the template, and the caller can decide
        // how to use that thing. Likely an enum that specifies if it's raw data or a
        // templated output
        output.opt_template(
            match succeed {
                true => self.config.template.as_ref(),
                false => self.config.template_failure.as_ref(),
            }
        )
        .with_context(|| format!("Your request was sent but the output or failure-template within could not be parsed, run with -v to see if your request was successful"))?
        .opt_template(match succeed {
            true => args.get_one("template"),
            false => args.get_one("failure-template"),
        })
        .with_context(|| format!("Your request was sent but the --template or --failure-template could not be parsed, run with -v to see if your request was successful"))?
        .opt_output(match succeed {
                true => self.config.output.as_ref(),
                false => self.config.output_failure.as_ref().or(self.config.output.as_ref())
            })
            .await.with_context(|| format!("could not set --output"))?
        .opt_output(match succeed {
            true => args.get_one("output"),
            false => args.get_one("output-failure").or(args.get_one("output")),
        })
        .await.with_context(|| format!("could not set --output"))?
        .render(response)
        .await.with_context(|| format!("could not write output to specified location!"))?;
        Ok(())
    }
}
