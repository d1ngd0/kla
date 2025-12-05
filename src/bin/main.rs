use std::{ffi::OsString, fs, sync::Arc};

use anyhow::{anyhow, Context as _};
use clap::{arg, command, ArgAction, ArgMatches, Command};
use kla::{
    clap::{arg_file_value, arg_file_writer, DefaultValueIfSome},
    config::{Config, ConfigCommand},
    Collection, Environment, KlaClientBuilder, KlaRequestBuilder, Opt, Optional, OutputBuilder,
    Sigv4Request, TemplateBuilder, When,
};
use log::{debug, error, info, trace, LevelFilter};
use regex::Regex;
use reqwest::{redirect::Policy, ClientBuilder, RequestBuilder, Response};
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
        .arg(arg!(-v --verbose "-v Warning, -vv Info, -vvv Debug, -vvvv Trace; not specified logs Error").action(ArgAction::Count))
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
        .arg(arg!(--"accept-invalid-certs" "Controls the use of certificate validation.").action(ArgAction::SetTrue).long_help("Warning

You should think very carefully before using this method. If invalid certificates are trusted, any certificate for any site will be trusted for use. This includes expired certificates. This introduces significant vulnerabilities, and should only be used as a last resort."))
        .arg(arg!(--"accept-invalid-hostnames" "Controls the use of hostname verification.").action(ArgAction::SetTrue).long_help("Warning

You should think very carefully before you use this method. If hostname verification is not used, any valid certificate for any site will be trusted for use from any other. This introduces a significant vulnerability to man-in-the-middle attacks."))
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

/// args_client parses the flags from the user and applies them to the client builder.
/// this function should be used when calling the `with_priority` functions on the environment
/// It is important that this spot not assume settings since it takes the highest priority
/// meaning "defaults" could overload set values in `default_settings` or a specified environment
pub fn args_client<'a>(
    args: &'a ArgMatches,
) -> impl Fn(ClientBuilder) -> kla::Result<ClientBuilder> + use<'a> {
    move |builder| {
        Ok(builder
            .use_rustls_tls()
            .with_some(args.get_one::<String>("agent"), ClientBuilder::user_agent)
            .when(args.get_raw("no-gzip").is_some(), |b| b.gzip(false))
            .when(args.get_raw("no-brotli").is_some(), |b| b.brotli(false))
            .when(args.get_raw("no-deflate").is_some(), |b| b.deflate(false))
            .when(args.get_count("verbose") >= 1, |b| {
                b.connection_verbose(true)
            })
            .when(args.get_raw("no-redirects").is_some(), |b| {
                b.redirect(Policy::none())
            })
            .with_some(args.get_one("max-redirects"), |b, redirects| {
                b.redirect(Policy::limited(*redirects))
            })
            .when(args.get_raw("accept-invalid-certs").is_some(), |b| {
                b.danger_accept_invalid_certs(true)
            })
            .when(args.get_raw("accept-invalid-hostnames").is_some(), |b| {
                b.danger_accept_invalid_hostnames(true)
            })
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
            .with_context(|| format!("could not add certificate"))?)
    }
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
    let config = Config::from_list(
        [
            "config.toml",
            "~/.kla.toml",
            "~/.config/kla/config.toml",
            "/etc/kla/config.toml",
        ]
        .iter(),
    )?;

    // let conf = Config::builder()
    //     .add_source(File::new(&config_file, FileFormat::Toml))
    //     .set_default("default.environment", "/etc/kla/.default-environment")?
    //     .build()
    //     .with_context(|| format!("could not load configuration"))?
    //     .merge_children("config")
    //     .context("could not load [[config]] files")?;

    // if the config file has a default environment we want to store it in a static
    // variable so it can be used everywhere
    if let Some(default_environment) = config
        .default_environment
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
    {
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
        .subcommand(
            Command::new("bulk")
            .about("Allows you to run multiple http requests all at once")
            .alias("collection")
            .arg(arg!(collection: [collection] "The collection or directory of collections you want to run"))
            .allow_external_subcommands(true)
            .disable_help_flag(true)
            .arg(
                arg!([args] ... "Any arguments for the collection")
                    .trailing_var_arg(true)
                    .allow_hyphen_values(true),
            )
            .arg(arg!(-h --help "Show the help text for collection or collection directory")),
        )
        .get_matches();

    colog::basic_builder()
        .filter_level(match m.get_count("verbose") {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
        .init();

    match m.subcommand() {
        Some(("environments", envs)) => run_environments(envs, &config),
        Some(("switch", envs)) => run_switch(envs, &config),
        Some(("run", envs)) => run_run(envs.get_one::<String>("template"), &m, &config).await,
        Some(("bulk", envs)) => {
            run_collection(envs.get_one::<String>("collection"), &m, &config).await
        }
        _ => run_root(&m, &config).await,
    }
}

/// run_run will exectute a template
async fn run_collection<S: Into<String>>(
    collection: Option<S>,
    args: &ArgMatches,
    conf: &Config,
) -> Result<(), anyhow::Error> {
    // Get the name of the template
    let collection: String = match collection.map(|s| s.into()) {
        None => return run_collection_empty(args, conf).await,
        Some(collection) if collection == "help" => return run_collection_empty(args, conf).await,
        Some(collection) if collection == "--help" => {
            return run_collection_empty(args, conf).await
        }
        Some(collection) => collection,
    };
    trace!("running collection {}", collection);

    let clct_config = match Collection::from_file(conf.collection_path(&collection)?.as_path()) {
        Ok(tmpl_config) => tmpl_config,
        Err(_) => return run_collection_empty(args, conf).await,
    };
    debug!("collection loaded {:#?}", clct_config);

    Ok(())
}

/// run_run will exectute a template
async fn run_run<S: Into<String>>(
    template: Option<S>,
    args: &ArgMatches,
    conf: &Config,
) -> Result<(), anyhow::Error> {
    // Get the name of the template
    let template: String = match template.map(|s| s.into()) {
        None => return run_run_empty(args, conf).await,
        Some(template) if template == "help" => return run_run_empty(args, conf).await,
        Some(template) if template == "--help" => return run_run_empty(args, conf).await,
        Some(template) => template,
    };
    trace!("running template {}", template);

    // Get the environment
    let env =
        Optional::from_config_with_priority(args.get_one::<String>("env"), conf, args_client(args))
            .await
            .with_context(|| {
                format!(
                    "could not load environment: {:?}",
                    args.get_one::<String>("env")
                )
            })?;
    debug!("Running under environment {:#?}", env);

    // Get the configuration for the template in the environment
    let tmpl_config = match ConfigCommand::from_file(env.tmpl_path(&template)?.as_path()) {
        Ok(tmpl_config) => tmpl_config,
        Err(_) => return run_run_empty(args, conf).await,
    };
    debug!("config loaded {:#?}", tmpl_config);

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

    let output = TemplateBuilder::new()
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
            args.get_one("dry").map(|b| *b).unwrap_or_default(),
        )
        .await?;

    let mut writer = match output.is_success() {
        true => {
            let writer = arg_file_writer(tmpl_config.output.as_ref(), "output")
                .await
                .transpose()?;

            if let Some(writer) = writer {
                writer
            } else {
                arg_file_writer(args.get_one::<String>("output"), "output")
                    .await
                    .transpose()?
                    .unwrap_or_else(|| Box::pin(tokio::io::stdout()))
            }
        }
        false => {
            let writer = arg_file_writer(tmpl_config.output.as_ref(), "output_failure")
                .await
                .transpose()?;

            if let Some(writer) = writer {
                writer
            } else {
                arg_file_writer(args.get_one::<String>("output-failure"), "output-failure")
                    .await
                    .transpose()?
                    .unwrap_or_else(|| Box::pin(tokio::io::stdout()))
            }
        }
    };

    output.copy(&mut writer).await?;

    Ok(())
}

async fn run_run_empty(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    debug!("no template, running templates with empty set");
    let env =
        Optional::from_config_with_priority(args.get_one::<String>("env"), conf, args_client(args))
            .await
            .with_context(|| {
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
        let tmpl_conf = ConfigCommand::from_file(env.tmpl_path(&template)?.as_path())
                        .with_context(|| format!("environment {:?} with tempalte {} could not be rendered as command, is something wrong with the template?", env.name(), &template))?;
        m = m.subcommand(Command::try_from(tmpl_conf)?);
    }

    command().subcommand(m).get_matches();

    Ok(())
}

async fn run_collection_empty(_args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    debug!("no collection, running collection with empty set");

    let mut m = Command::new("bulk")
        .about("run collections")
        .alias("collection")
        .arg_required_else_help(true);

    let collections = conf.collections().with_context(|| {
        format!(
            "could not fetch all collections from {:?}",
            conf.collection_dir.as_ref()
        )
    })?;

    for collection in collections {
        let tmpl_conf = Collection::from_file(conf.collection_path(&collection)?.as_path())
            .with_context(|| {
                format!(
                    "collection {} could not be rendered as command",
                    &collection
                )
            })?;
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
        .environments()
        .filter(|endpoint| r.is_match(&endpoint.name));

    for endpoint in environments {
        println!("{}", endpoint);
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
        .environments()
        .filter(|endpoint| r.is_match(&endpoint.name));

    let mut num_entries = 0;
    for endpoint in environments {
        let endpoint: Arc<dyn SkimItem> = Arc::new(endpoint.clone());
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

    if let Some(selected) = selected.as_ref() {
        match conf.default_environment.as_ref() {
            Some(file_path) => fs::write(file_path, selected).with_context(|| {
                format!("could not write current environment file to {}", file_path)
            }),
            None => Err(anyhow!("config file does not specify a default_environment value in the root. This file stores the selected environment. Try adding `default_environment = \"~/.config/kla/.env\"` to your kla config file.")),
        }?;
        println!("Switched to environment {}", &selected)
    }

    Ok(())
}

// run_root will run the command with no arguments
async fn run_root(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let env =
        Optional::from_config_with_priority(args.get_one::<String>("env"), conf, args_client(args))
            .await
            .with_context(|| {
                format!(
                    "could not load environment: {:?}",
                    args.get_one::<String>("env")
                )
            })?;
    info!("Environment: {}", env.name());
    debug!("{:#?}", env);

    let (uri, method) = if let Some(uri) = args.get_one::<String>("url") {
        (
            uri,
            args.get_one::<String>("method-or-url")
                .expect("required")
                .to_uppercase(),
        )
    } else {
        (
            args.get_one("method-or-url").expect("required"),
            "GET".into(),
        )
    };
    info!("uri request [{}] {}", method, uri);

    let request = env
        .request(method.as_str(), uri)?
        .with_some(
            arg_file_value(args.get_one("body"), "body")?,
            RequestBuilder::body,
        )
        .opt_headers(args.get_many("header"))
        .with_context(|| {
            format!(
                "could not set header: {:?}",
                args.get_many::<String>("header")
            )
        })?
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
        .context("Could not build http request")
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
    info!("{:#?}", request);

    let response = match args.get_one("dry").map(|b| *b).unwrap_or_default() {
        true => Response::from(http::Response::<Vec<u8>>::default()),
        false => env
            .execute(request)
            .await
            .with_context(|| format!("request failed!"))?,
    };

    let succeed = response.status().is_success();
    info!("{:#?}", response);

    let output = OutputBuilder::new().with_some_result(match succeed {
            true => args.get_one::<String>("template"),
            false => args.get_one::<String>("failure-template"),
        }, OutputBuilder::template)
        .with_context(|| format!("Your request was sent but the --template or --failure-template could not be parsed, run with -v to see if your request was successful"))?
        .build(response)
        .await.with_context(|| format!("could not write output to specified location!"))?;

    let mut writer = match succeed {
        true => arg_file_writer(args.get_one::<String>("output"), "output")
            .await
            .transpose()?
            .unwrap_or_else(|| Box::pin(tokio::io::stdout())),
        false => arg_file_writer(args.get_one::<String>("failure-output"), "output")
            .await
            .transpose()?
            .unwrap_or_else(|| Box::pin(tokio::io::stderr())),
    };

    output.copy(&mut writer).await?;

    Ok(())
}
