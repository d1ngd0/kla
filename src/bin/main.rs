use std::{ffi::OsString, fs, path::Path, sync::Arc};

use anyhow::Context as _;
use clap::{arg, command, ArgAction, ArgMatches, Command};
use config::{Config, File, FileFormat};
use http::Method;
use kla::{
    clap::DefaultValueIfSome,
    config::{ConfigCommand, MergeChildren},
    Endpoint, Environment, Expand, KlaClientBuilder, KlaRequestBuilder, Optional, OutputBuilder,
    Sigv4Request, TemplateBuilder, URLBuilder, When, WithEnvironment,
};
use log::error;
use regex::Regex;
use reqwest::{ClientBuilder, Response};
use skim::{prelude::SkimOptionsBuilder, Skim, SkimItem};
use tokio::sync::OnceCell;

static DEFAULT_ENV: OnceCell<OsString> = OnceCell::const_new();

static ROOT_ABOUT: &'static str = include_str!("txt/root_about.txt");
static RUN_ABOUT: &'static str = include_str!("txt/run_about.txt");

fn command() -> Command {
    command!()
        .arg_required_else_help(true)
        .long_about(ROOT_ABOUT)
        .subcommand_required(false)
        .arg(arg!(--agent <AGENT> "The header agent string").default_value("kla"))
        .arg(arg!(-e --env <ENVIRONMENT> "The environment we will run the request against").required(false).default_value_if_some(DEFAULT_ENV.get().map(|v| v.as_os_str())))
        .arg(arg!(-t --template <TEMPLATE> "The template to use when formating the output. prepending with @ will read a file."))
        .arg(arg!(--"failure-template" <TEMPLATE> "The template to use when formating the failure output. prepending with @ will read a file."))
        .arg(arg!(-o --output <FILE> "The file to write the output into"))
        .arg(arg!(--"output-failure" <FILE> "Where any failure will be written out to"))
        .arg(arg!(--timeout <SECONDS> "The amount of time allotted for the request to finish"))
        .arg(arg!(--"basic-auth" <BASIC_AUTH> "The username and password seperated by :, a preceding @ denotes a file path."))
        .arg(arg!(--"bearer-token" <BEARER_TOKEN> "The bearer token to use in requests. A preceding @ denotes a file path."))
        .arg(arg!(-H --header <HEADER> "Specify a header The key and value should be seperated by a : (eg --header \"Content-Type: application/json\")").action(ArgAction::Append))
        .arg(arg!(-Q --query <QUERY> "Specify a query parameter The key and value should be seperated by a = (eg --query \"username=Jed\")").action(ArgAction::Append))
        .arg(arg!(-F --form <FORM> "Specify a form key=value to be passed in the form body").action(ArgAction::Append))
        .arg(arg!(-v --verbose "make it loud and proud").action(ArgAction::SetTrue))
        .arg(arg!(--dry "don't actually do anything, will automatically enable verbose").action(ArgAction::SetTrue))
        .arg(arg!(--"http-version" <HTTP_VERSION> "The version of http to send the request as").value_parser(["0.9", "1.0", "1.1", "2.0", "3.0"]))
        .arg(arg!(--"no-gzip" "Do not automatically uncompress gzip responses").action(ArgAction::SetTrue))
        .arg(arg!(--"no-brotli" "Do not automatically uncompress brotli responses").action(ArgAction::SetTrue))
        .arg(arg!(--"no-deflate" "Do not automatically uncompress deflate responses").action(ArgAction::SetTrue))
        .arg(arg!(--"max-redirects" <NUMBER> "The number of redirects allowed"))
        .arg(arg!(--"no-redirects" "Disable any redirects").action(ArgAction::SetTrue))
        .arg(arg!(--proxy <PROXY> "The proxy to use for all requests."))
        .arg(arg!(--"proxy-http" <PROXY_HTTP> "The proxy to use for http requests."))
        .arg(arg!(--"proxy-https" <PROXY_HTTPS> "The proxy to use for https requests."))
        .arg(arg!(--"proxy-auth" <PROXY_AUTH> "The username and password seperated by :."))
        .arg(arg!(--"connect-timeout" <DURATION> "The amount of time to allow for connection"))
        .arg(arg!(--"sigv4" "Sign the request with AWS v4 Signature").action(ArgAction::SetTrue))
        .arg(arg!(--"sigv4-aws-profile" <AWS_PROFILE> "The AWS profile to use when signing a request"))
        .arg(arg!(--"sigv4-service" <SERVICE> "The AWS Service to use when signing the request"))
        .arg(arg!(--certificate <CERTIFICATE_FILE> "The path to the certificate to use for requests. Accepts PEM and DER, expects files to end in .der or .pem. defaults to pem").action(ArgAction::Append))
        .arg(arg!("method-or-url": [METHOD_OR_URL] "The URL path (with an assumed GET method) OR the method if another argument is supplied"))
        .arg(arg!(url: [URL] "The URL path when a method is supplied"))
        .arg(arg!(body: [BODY] "The body of the HTTP request, if prefixed with a `@` it is treated as a file path"))
        .subcommand(
            Command::new("environments")
            .about("Show the environments that are available to you.")
            .alias("envs")
            .arg(arg!(-r --regex <STATEMENT> "A regex statement").required(false).default_value(".*"))
        )
        .subcommand(
            Command::new("switch")
            .about("Select an environment to be the current context")
            .alias("context")
            .arg(arg!(matcher: [Matcher] "A regex statement to filter down matches, (if we only match one value it's selected)").required(false).default_value(".*"))
        )
}

fn args_client(args: &ArgMatches) -> Result<ClientBuilder, anyhow::Error> {
    let client_builder = ClientBuilder::new()
        .opt_header_agent(args.get_one("agent"))
        .with_context(|| format!("could not add agent: {:?}", args.get_one::<String>("agent")))?
        .gzip(
            !args
                .get_one::<bool>("no-gzip")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .brotli(
            !args
                .get_one::<bool>("no-brotli")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .deflate(
            !args
                .get_one::<bool>("no-deflate")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .connection_verbose(
            args.get_one::<bool>("verbose")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .opt_max_redirects(args.get_one("max-redirects"))
        .no_redirects(
            args.get_one::<bool>("no-redirects")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .opt_proxy(args.get_one("proxy"), args.get_one("proxy-auth"))
        .with_context(|| {
            format!(
                "could not add proxy: --proxy={:?} --proxy-auth={:?}",
                args.get_one::<String>("proxy"),
                args.get_one::<String>("proxy-auth")
                    .map(|v| "*".repeat(v.len()))
            )
        })?
        .opt_proxy_http(args.get_one("proxy-http"), args.get_one("proxy-auth"))
        .with_context(|| {
            format!(
                "could not add proxy: --proxy-http={:?} --proxy-auth={:?}",
                args.get_one::<String>("proxy-http"),
                args.get_one::<String>("proxy-auth")
                    .map(|v| "*".repeat(v.len()))
            )
        })?
        .opt_proxy_https(args.get_one("proxy-https"), args.get_one("proxy-auth"))
        .with_context(|| {
            format!(
                "could not add proxy: --proxy-https={:?} --proxy-auth={:?}",
                args.get_one::<String>("proxy-https"),
                args.get_one::<String>("proxy-auth")
                    .map(|v| "*".repeat(v.len()))
            )
        })?
        .opt_certificate(args.get_many("certificate"))
        .with_context(|| format!("could not add certificate"))?;
    Ok(client_builder)
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => (),
        Err(err) => error!(
            "{}",
            err.chain().fold(String::new(), |mut f, err| {
                f.push_str(err.to_string().as_str());
                f.push_str("\n");
                f
            })
        ),
    }
}

async fn run() -> Result<(), anyhow::Error> {
    colog::init();

    let config_file = [
        "config.toml".into(),
        "~/.kla.toml".shell_expansion(),
        "~/.config/kla/config.toml".shell_expansion(),
        "/etc/kla/config.toml".into(),
    ]
    .into_iter()
    .filter(|f| Path::new(f).exists())
    .next()
    .ok_or(anyhow::Error::msg("No valid config file found"))?;

    let conf = Config::builder()
        .add_source(File::new(&config_file, FileFormat::Toml))
        .set_default("default.environment", "/etc/kla/.default-environment")?
        .build()
        .with_context(|| format!("could not load configuration"))?
        .merge_children("config")
        .context("could not load [[config]] files")?;

    // if the config file has a default environment we want to store it in a static
    // variable so it can be used everywhere
    if let Ok(default_environment) = fs::read_to_string(
        conf.get_string("default.environment")
            .map(String::shell_expansion)
            .expect("default value"),
    ) {
        DEFAULT_ENV
            .get_or_init(|| async { OsString::from(default_environment) })
            .await;
    }

    let m = command()
        .subcommand(
            Command::new("run")
                .about("run templates defined for the environment")
                .long_about(RUN_ABOUT)
                .alias("template")
                .arg(arg!(template: [template] "The template you want to run"))
                .allow_external_subcommands(true)
                .disable_help_flag(true)
                .arg(
                    arg!([args] ... "Any arguments for the template")
                        .trailing_var_arg(true)
                        .allow_hyphen_values(true),
                )
                .arg(arg!(-h --help "Show the help command, and all templates available to you.")),
        )
        .get_matches();

    match m.subcommand() {
        Some(("environments", envs)) => run_environments(envs, &conf),
        Some(("switch", envs)) => run_switch(envs, &conf),
        Some(("run", envs)) => run_run(envs.get_one::<String>("template"), &m, &conf).await,
        _ => run_root(&m, &conf).await,
    }
}

/// run_run will exectute a template
async fn run_run<S: Into<String>>(
    template: Option<S>,
    args: &ArgMatches,
    conf: &Config,
) -> Result<(), anyhow::Error> {
    // Get the name of the template
    let template: String = match template.map(|s| s.into()) {
        None => return run_run_empty(args, conf),
        Some(template) if template == "help" => return run_run_empty(args, conf),
        Some(template) if template == "--help" => return run_run_empty(args, conf),
        Some(template) => template,
    };

    // Get the environment
    // TODO: you were here, you were going to make this into a function on option
    // to build the optional environment from these arguments
    let env = Optional::from_config(args.get_one("env"), conf).with_context(|| {
        format!(
            "could not load environment: {:?}",
            args.get_one::<String>("env")
        )
    })?;

    // Get the configuration for the template in the environment
    let tmpl_config = match Config::builder()
        .add_source_environment(&env, &template)
        .with_context(|| {
            format!(
                "could not load {} for environment {:?}",
                &template,
                env.name(),
            )
        })?
        .build()
    {
        Ok(tmpl_config) => ConfigCommand::with_name(&template, tmpl_config)?,
        Err(_) => return run_run_empty(args, conf),
    };

    // Run the command parsing for the template again, this will make actually
    // parse things with the configured arguments etc
    let m = command()
        .subcommand(
            Command::new("run")
                .about("run templates defined for the environment")
                .long_about(RUN_ABOUT)
                .alias("template")
                .subcommand(Command::try_from(tmpl_config.clone())?),
        )
        .get_matches();

    TemplateBuilder::new()
        .client(args_client(&m)?.with_environment(&env).await?.build()?)
        // TODO: This should be changed to try_config, and we shouldn't turn it into
        // a ConfigCommand here, all that should be done inside the builder
        // We will need to get the name in the config somehow
        .config(tmpl_config.clone())
        .build()?
        .run(
            &env,
            m.subcommand()
                .expect("only run in run")
                .1
                .subcommand()
                .expect("only run with template")
                .1,
        )
        .await?;
    Ok(())
}

fn run_run_empty(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let env = Environment::new(args.get_one("env"), conf).with_context(|| {
        format!(
            "could not load environment: {:?}",
            args.get_one::<String>("env")
        )
    })?;

    let mut m = Command::new("run")
        .about("run templates defined for the environment")
        .long_about(RUN_ABOUT)
        .alias("template")
        .arg_required_else_help(true);
    let templates = env.templates().with_context(|| {
        format!(
            "could not fetch all templates for {:?} from {:?}",
            env.name(),
            env.template_dir()
        )
    })?;

    for template in templates {
        let tmpl_conf = Config::builder()
            .add_source_environment(&env, &template)
            .and_then(|b| Ok(b.build()?))
            .and_then(|conf| ConfigCommand::with_name(&template, conf))
            .with_context(|| format!("environment {:?} with tempalte {} could not be rendered as command, is something wrong with the template?", env.name(), &template))?;
        m = m.subcommand(Command::try_from(tmpl_conf)?);
    }

    command().subcommand(m).get_matches();

    Ok(())
}

fn run_environments(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let r = Regex::new(args.get_one::<String>("regex").unwrap()).with_context(|| {
        format!(
            "invalid regex supplied {:?}",
            args.get_one::<String>("regex")
        )
    })?;

    let environments = conf
        .get_table("environment")
        .with_context(|| format!("Could not load environments from config"))?
        .into_iter()
        .filter_map(|(k, v)| if r.is_match(&k) { Some((k, v)) } else { None });

    for (k, v) in environments {
        let env: Endpoint = v
            .try_deserialize()
            .with_context(|| format!("invalid endpoint {}", k))?;
        println!("{}", env);
    }

    Ok(())
}

fn run_switch(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let (send, recv) = crossbeam_channel::unbounded();
    let r = Regex::new(args.get_one::<String>("matcher").unwrap()).with_context(|| {
        format!(
            "invalid regex supplied {:?}",
            args.get_one::<String>("regex")
        )
    })?;

    let environments = conf
        .get_table("environment")
        .with_context(|| format!("Could not load environments from config"))?
        .into_iter()
        .filter_map(|(k, v)| if r.is_match(&k) { Some((k, v)) } else { None });

    let mut num_entries = 0;
    for (name, val) in environments {
        let mut endpoint: Endpoint = val
            .try_deserialize()
            .with_context(|| format!("invalid endpoint {}", name))?;
        endpoint.name = name;
        let endpoint: Arc<dyn SkimItem> = Arc::new(endpoint);
        send.send(endpoint).unwrap();

        num_entries += 1;
    }

    let options = SkimOptionsBuilder::default()
        .preview(Some(String::from("right")))
        .build()?;

    let selected = match num_entries {
        0 => {
            return Err(anyhow::Error::msg(format!(
                "no environments exist that match your filter: {}",
                r
            )))
        }
        1 => Some(
            recv.recv()
                .context("could not recieve environment from channel")?
                .text()
                .to_string(),
        ),
        _ => Skim::run_with(&options, Some(recv))
            .filter(|f| !f.is_abort)
            .map(|v| v.selected_items)
            .into_iter()
            .flatten()
            .next()
            .map(|v| v.text().to_string()),
    };

    let environment_file = conf
        .get_string("default.environment")
        .map(String::shell_expansion)
        .expect("default value");

    if let Some(selected) = selected {
        fs::write(&environment_file, &selected).with_context(|| {
            format!(
                "could not write current environment file to {}",
                &environment_file
            )
        })?;
        println!("Switched to environment {}", &selected)
    }

    Ok(())
}

// run_root will run the command with no arguments
async fn run_root(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let env = Environment::new(args.get_one("env"), conf).with_context(|| {
        format!(
            "could not load environment: {:?}",
            args.get_one::<String>("env")
        )
    })?;

    let verbose = args
        .get_one::<bool>("verbose")
        .map(|v| *v)
        .unwrap_or_default();

    let (uri, method) = if let Some(uri) = args.get_one::<String>("url") {
        (
            uri,
            Method::try_from(
                args.get_one::<String>("method-or-url")
                    .expect("required")
                    .to_uppercase()
                    .as_str(),
            )
            .with_context(|| {
                format!(
                    "{:?} is not a valid method",
                    args.get_one::<String>("method-or-url")
                )
            })?,
        )
    } else {
        (
            args.get_one("method-or-url").expect("required"),
            Method::GET,
        )
    };

    let url = env.url_builder().build(uri)?;
    let client = args_client(args)?.with_environment(&env).await?.build()?;

    let request = client
        .request(method, url)
        .with_environment(&env)
        .await?
        .opt_body(args.get_one("body"))
        .with_context(|| format!("could not set body: {:?}", args.get_one::<String>("body")))?
        .opt_headers(args.get_many("header"))
        .with_context(|| {
            format!(
                "could not set header: {:?}",
                args.get_many::<String>("header")
            )
        })?
        .opt_bearer_auth(args.get_one("bearer-token"))
        .opt_basic_auth(args.get_one("basic-auth"))
        .opt_query(args.get_many("query"))
        .with_context(|| {
            format!(
                "could not set query param: {:?}",
                args.get_many::<String>("query")
            )
        })?
        .opt_form(args.get_many("form"))
        .with_context(|| {
            format!(
                "could not set form param: {:?}",
                args.get_many::<String>("form")
            )
        })?
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
        .context("Could not build http request")?
        .with_environment(&env)
        .await?;

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

    let output = OutputBuilder::new().when(verbose, |builder| builder.request_prelude(&request));

    let response = match args.get_one("dry").map(|b| *b).unwrap_or_default() {
        true => Response::from(http::Response::<Vec<u8>>::default()),
        false => client
            .execute(request)
            .await
            .with_context(|| format!("request failed!"))?,
    };

    let succeed = response.status().is_success();

    output.opt_template(if succeed {
            args.get_one("template")
        } else {
            args.get_one("failure-template")
        })
        .with_context(|| format!("Your request was sent but the --template or --failure-template could not be parsed, run with -v to see if your request was successful"))?
        .when(verbose, |builder| builder.response_prelude(&response))
        .opt_output(args.get_one("output"))
        .await
        .with_context(|| format!("could not set --output"))?
        .render(response)
        .await.with_context(|| format!("could not write output to specified location!"))?;

    Ok(())
}
