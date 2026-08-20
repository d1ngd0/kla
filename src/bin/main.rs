use std::{
    env,
    fs::{self, OpenOptions},
    io::{stdout, Write},
    path::PathBuf,
    process::exit,
    str::from_utf8,
    sync::Arc,
};

use anyhow::{anyhow, Context as _};
use clap::{arg, command, ArgAction, ArgMatches, Command};
use kla::{
    clap::{arg_file_writer, ArgOptions},
    config::{Attributes, CollectionConfig, Config, ConfigCommand},
    AsyncOption, AsyncResult as _, CollectionBuilder, Environment, Error, ExtensionRepo,
    KlaRequest, Opt, Optional, OutputBuilder, Sigv4Request, TemplateBuilder, When,
};
use log::{debug, error, info, trace, LevelFilter};
use oci_client::Reference;
use regex::Regex;
use reqwest::Response;
use skim::{prelude::SkimOptionsBuilder, Skim, SkimItem};
use tokio::sync::OnceCell;

static ENV: OnceCell<String> = OnceCell::const_new();

static ROOT_ABOUT: &'static str = include_str!("txt/root_about.txt");
static RUN_ABOUT: &'static str = include_str!("txt/run_about.txt");
static COLLECTION_ABOUT: &'static str = include_str!("txt/run_about.txt");
static DEFAULT_CONFIG: &'static str = include_str!("../../kla.toml");

fn command() -> Command {
    Attributes::clap_flags(command!())
        .arg_required_else_help(true)
        .long_about(ROOT_ABOUT)
        .subcommand_required(false)
        .arg(arg!(--config <CONFIG_FILE> "The configuration file to use"))
        .arg(arg!(-e --env <ENVIRONMENT> "The environment we will run the request against").required(false))
        .arg(arg!(-t --template <TEMPLATE> "The template to use when formating the output. prepending with @ will read a file."))
        .arg(arg!(--"failure-template" <TEMPLATE> "The template to use when formating the failure output. prepending with @ will read a file."))
        .arg(arg!(-o --output <FILE> "The file to write the output into"))
        .arg(arg!(--"output-failure" <FILE> "Where any failure will be written out to"))
        .arg(arg!(-H --header <HEADER> "Specify a header The key and value should be seperated by a : (eg --header \"Content-Type: application/json\")").action(ArgAction::Append))
        .arg(arg!(-Q --query <QUERY> "Specify a query parameter The key and value should be seperated by a = (eg --query \"username=Jed\")").action(ArgAction::Append))
        .arg(arg!(-F --form <FORM> "Specify a form key=value to be passed in the form body").action(ArgAction::Append))
        .arg(arg!(-v --verbose "-v Warning, -vv Info, -vvv Debug, -vvvv Trace; not specified logs Error").action(ArgAction::Count))
        .arg(arg!(--dry "don't actually do anything, will automatically enable verbose").action(ArgAction::SetTrue))
        .arg(arg!(--edit "edit the body of the request before sending it off").action(ArgAction::SetTrue))
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
        .subcommand(
            Command::new("environment")
            .alias("env")
        )
        .subcommand(
            Command::new("init")
            .arg(arg!(--force "ignore if the file exists, truncate it and write the default config again").action(ArgAction::SetTrue))
            .about("Create the initial config file")
        )
        .subcommand(
            Command::new("extension")
            .subcommand_required(true)
            .alias("ext")
            .subcommand(
                Command::new("list")
                .about("list all the extensions that are installed.")
                .arg(arg!(--matches <regex> "Only show the extensions which match the supplied regex"))
                .arg(arg!(--locked "Only show the extensions that are locked"))
                .arg(arg!(--"name-only" "Only list the name, ommiting the directory of the installed extension"))
                .arg(arg!(--"dir-only" "Only list the directory, ommiting the registry and version of the installed extension"))
            )
            .subcommand(
                Command::new("lock")
                .about("lock an extension to the currently installed or supplied version")
                .arg(arg!(<image> "The OCI extension path that you want to pull in").num_args(1..))
                .arg(arg!(--unlock "unlock the new extension to enable updates").action(ArgAction::SetTrue))
            )
            .subcommand(
                Command::new("add")
                .about("adds a new extension")
                .arg(arg!(<image> "The OCI extension path that you want to pull in").num_args(1..))
                .arg(arg!(--lock "lock the new extension to the installed version"))
            )
            .subcommand(
                Command::new("remove")
                .about("removes the supplied extion")
                .arg(arg!(<image> "The OCI extension path that you want to remove").num_args(1..))
            )
            .subcommand(
                Command::new("update")
                .about("updates your existing extensions")
            )
        )
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => (),
        Err(err) => {
            println!("{:#}", err);
            exit(1);
        }
    }
}

async fn run() -> Result<(), anyhow::Error> {
    let mut m = command()
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

    let mut config = if let Some(path) = m.get_one::<String>("config") {
        Config::from_path(path)?
    } else {
        Config::from_list(
            [
                "kla.toml",
                ".kla.toml",
                "~/.kla.toml",
                "~/.config/kla/config.toml",
                "/etc/kla/config.toml",
            ]
            .iter(),
        )?
    };
    log::debug!("Config Contents: {:?}", config);

    let repo = ExtensionRepo::try_from(&config.extensions)?;
    repo.apply(&mut config)?;

    // check the env flag, and then the default config for the correct
    // environment to use. If you ever need to get the environment
    // use this onecell instead of checking the argument
    if let Some(env) = m.get_one::<String>("env") {
        ENV.get_or_init(|| async { env.into() }).await;
    } else if let Some(default_environment) = config
        .default_environment
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
    {
        ENV.get_or_init(|| async { default_environment }).await;
    }
    // make sure we can't use it
    m.remove_one::<String>("env");

    colog::basic_builder()
        .filter_level(match m.get_count("verbose") {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
        .init();

    let r = match m.subcommand() {
        Some(("environments", envs)) => run_environments(envs, &config),
        Some(("switch", envs)) => run_switch(envs, &config),
        Some(("run", envs)) => {
            run_run(
                envs.get_one::<String>("template"),
                &m,
                &config,
                Attributes::from(&m),
            )
            .await
        }
        Some(("bulk", envs)) => {
            run_collection(
                envs.get_one::<String>("collection"),
                &m,
                &config,
                Attributes::from(&m),
            )
            .await
        }
        Some(("environment", envs)) => run_environment(envs, &config),
        Some(("init", envs)) => run_init(envs),
        Some(("extension", envs)) => run_extensions(envs, &config).await,
        _ => run_root(&m, &config, Attributes::from(&m)).await,
    };

    if let Err(err) = r {
        log::error!("{:#}", err);
        exit(1);
    }
    Ok(())
}

fn run_init(envs: &ArgMatches) -> Result<(), anyhow::Error> {
    let dir = match env::home_dir() {
        Some(home) => home.join(".config/kla/"),
        None => PathBuf::from("/etc/kla/"),
    };
    let config_file = dir.join("config.toml");

    // Check if the file already exists and only overwrite it if
    if !config_file.exists()
        || envs
            .get_one::<bool>("force")
            .map(|f| *f)
            .unwrap_or_default()
    {
        // Create the configuration directory
        fs::create_dir_all(&dir)?;
        // Write the contents of the config file
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&config_file)?
            .write(DEFAULT_CONFIG.as_bytes())?;
        // Create the conf.d directory we reference
        fs::create_dir_all(dir.join("conf.d"))?;

        println!("Created config file at {}", config_file.to_string_lossy());
    } else {
        println!(
            "Config {} already exists. Overwrite with --force",
            config_file.to_string_lossy()
        );
    }

    Ok(())
}

/// run_run will exectute a template
async fn run_collection<S: Into<String>>(
    collection: Option<S>,
    args: &ArgMatches,
    conf: &Config,
    _attrs: Attributes,
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

    let clct_config =
        match CollectionConfig::from_file(conf.collection_path(&collection)?.as_path()) {
            Ok(clct_config) => clct_config,
            Err(_) => return run_collection_empty(args, conf).await,
        };
    debug!("collection loaded {:#?}", clct_config);

    // Run the command parsing for the template again, this will make actually
    // parse things with the configured arguments etc
    let m = command()
        .subcommand(
            Command::new("bulk")
                .about("run templates defined for the environment")
                .long_about(COLLECTION_ABOUT)
                .alias("collection")
                .subcommand(Command::try_from(clct_config.clone())?),
        )
        .get_matches();

    let _collection = CollectionBuilder::new(conf)
        .config(&clct_config)
        .build()?
        .run(
            m.subcommand()
                .expect("only run as bulk")
                .1
                .subcommand()
                .expect("only run with collection")
                .1,
            args.get_one("dry").map(|b| *b).unwrap_or_default(),
        )
        .await?;

    Ok(())
}

/// run_run will exectute a template
async fn run_run<S: Into<String>>(
    template: Option<S>,
    args: &ArgMatches,
    conf: &Config,
    attrs: Attributes,
) -> Result<(), anyhow::Error> {
    // Get the name of the template
    let template: String = match template.map(|s| s.into()) {
        None => return run_run_empty(args, conf, attrs).await,
        Some(template) if template == "help" => return run_run_empty(args, conf, attrs).await,
        Some(template) if template == "--help" => return run_run_empty(args, conf, attrs).await,
        Some(template) => template,
    };
    trace!("running template {}", template);

    // Get the environment
    let env = Optional::from_config_with_priority(ENV.get(), conf, &attrs)
        .await
        .with_context(|| format!("could not load environment: {:?}", ENV.get()))?;
    debug!("Running under environment {:#?}", env);

    // Get the configuration for the template in the environment
    let tmpl_config = match ConfigCommand::from_file(env.tmpl_path(&template)?.as_path()) {
        Ok(tmpl_config) => tmpl_config,
        Err(_) => return run_run_empty(args, conf, attrs).await,
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

    // We need to find the leaf node of the command, if we are calling the root we
    // will set these and never loop. Otherwise we will traverse down by updating these
    // until we find the leaf
    let mut subcommand = m
        .subcommand()
        .expect("only run in run")
        .1
        .subcommand()
        .expect("only run with template")
        .1;
    let mut subcommand_config = &tmpl_config;

    // time to loop down
    loop {
        if let Some(tmp) = subcommand.subcommand() {
            subcommand = tmp.1;
            subcommand_config = subcommand_config
                .subcommands
                .iter()
                .find(|i| i.name == tmp.0)
                .expect("help would have fired if we couldn't find this")
        } else {
            break;
        }
    }

    let output = TemplateBuilder::new()
        .config(subcommand_config.clone())
        .build()?
        .run(&env, subcommand, args)
        .await?;

    let mut writer = arg_file_writer(output.desired_location(), "output")
        .await
        .async_or_else(async || {
            let argument = match output.is_success() {
                true => "output",
                false => "output-failure",
            };
            arg_file_writer(args.get_one::<String>(argument), argument).await
        })
        .await
        .transpose()?
        .unwrap_or_else(|| Box::pin(tokio::io::stdout()));

    output.copy(&mut writer).await?;

    Ok(())
}

async fn run_run_empty(
    _args: &ArgMatches,
    conf: &Config,
    attrs: Attributes,
) -> Result<(), anyhow::Error> {
    debug!("no template, running templates with empty set");
    let env = Optional::from_config_with_priority(ENV.get(), conf, &attrs)
        .await
        .with_context(|| format!("could not load environment: {:?}", ENV.get()))?;

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
        let tmpl_conf = CollectionConfig::from_file(conf.collection_path(&collection)?.as_path())
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
                format!("could not write current environment file to {:#?}", file_path)
            }),
            None => Err(anyhow!("config file does not specify a default_environment value in the root. This file stores the selected environment. Try adding `default_environment = \"~/.config/kla/.env\"` to your kla config file.")),
        }?;
        println!("Switched to environment {}", &selected)
    }

    Ok(())
}

// run_root will run the command with no arguments
async fn run_root(
    args: &ArgMatches,
    conf: &Config,
    attrs: Attributes,
) -> Result<(), anyhow::Error> {
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

    // check if the beginning of the uri is an environment or not. If the url
    // does not begin with `/` then we should assume the firsts value is the
    // environment name, and not a url. When specifying --env this value gets
    // ignored
    debug!("specified uri {:?}", uri);
    let (env, uri) = match uri.get(0..1) {
        Some("/") => (None, uri.clone()),
        Some(_) => {
            let mut parts = uri.splitn(2, "/");
            (
                parts.next().map(String::from),
                parts.next().map(String::from).unwrap_or(String::from("/")),
            )
        }
        None => (None, uri.clone()),
    };
    debug!("extracted environment: {:?}", env);

    let env = Optional::from_config_with_priority(ENV.get(), conf, &attrs)
        .await
        .with_context(|| format!("could not load environment: {:?}", ENV.get()))?;
    info!("Environment: {}", env.name());
    info!("uri request <{:?}> [{}] {}", env, method, uri);

    let request = env
        .request(method.as_str(), uri)?
        .with_arg_opts(args)?
        .build()
        .map_err(Error::from)
        .and_then(|req| env.sign(req))
        .when(args.get_count("verbose") > 0, |f| {
            f.inspect(|r| {
                info!(
                    "{}",
                    r.body()
                        .map(|f| f.as_bytes().unwrap_or_default())
                        .map(|f| from_utf8(f).unwrap_or_default())
                        .unwrap_or_default()
                );
            })
        })
        .async_and_then(async |req| req.edit(args.get_one::<bool>("edit")).await)
        .await
        .context("Could not build http request")?;

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

// run_environment prints the current default environment selected for kla
fn run_environment(_args: &ArgMatches, _conf: &Config) -> Result<(), anyhow::Error> {
    println!("{}", ENV.get().map(|s| s.as_str()).unwrap_or_default());

    Ok(())
}

//
async fn run_extensions(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    match args.subcommand() {
        Some(("add", m)) => run_extension_add(m, conf).await,
        Some(("remove", m)) => run_extension_remove(m, conf),
        Some(("update", m)) => run_extension_update(m, conf).await,
        Some(("list", m)) => run_extension_list(m, conf),
        Some(("lock", m)) => run_extension_lock(m, conf).await,
        _ => Ok(()),
    }
}

fn run_extension_list(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let repo = ExtensionRepo::try_from(&conf.extensions)?;
    let ext_match = Regex::new(
        args.get_one::<String>("matches")
            .map(|v| v.as_str())
            .unwrap_or(".*"),
    )?;
    let locked_only = args.get_one::<bool>("locked").copied().unwrap_or_default();

    for extension in repo.extensions()?.iter() {
        if !ext_match.is_match(&extension.remote.to_string()) {
            continue;
        }

        if locked_only && !extension.lock {
            continue;
        }

        if args
            .get_one::<bool>("name-only")
            .copied()
            .unwrap_or_default()
        {
            println!("{}", extension.remote);
        } else if args
            .get_one::<bool>("dir-only")
            .copied()
            .unwrap_or_default()
        {
            println!("{}", extension.dir.to_string_lossy());
        } else {
            println!("{}", extension);
        }
    }

    Ok(())
}

async fn run_extension_update(_args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let ext_repo = ExtensionRepo::try_from(&conf.extensions)?;

    for extension in ext_repo.extensions()?.iter() {
        if extension.lock {
            continue;
        }

        match ext_repo.update(extension, &mut stdout()).await {
            Ok(_) => println!("{} up to date", extension.remote),
            Err(err) => error!("{}: {}", extension.remote, err),
        }
    }

    Ok(())
}

fn run_extension_remove(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let ext_repo = ExtensionRepo::try_from(&conf.extensions)?;
    let log = &mut stdout();

    let extensions = args
        .get_many::<String>("image")
        .expect("clap to require")
        .map(&String::as_str)
        .map(Reference::try_from)
        .map(|f| f.context("parsing image path"))
        .collect::<Result<Vec<Reference>, anyhow::Error>>()?;

    for extension in extensions {
        ext_repo.remove(&extension, log)?;
    }
    Ok(())
}

async fn run_extension_add(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let ext_repo = ExtensionRepo::try_from(&conf.extensions)?;

    let lock = args.get_one::<bool>("lock").copied().unwrap_or_default();
    let log = &mut stdout();

    let extensions = args
        .get_many::<String>("image")
        .expect("clap to require")
        .map(&String::as_str)
        .map(Reference::try_from)
        .map(|f| f.context("parsing image path"))
        .collect::<Result<Vec<Reference>, anyhow::Error>>()?;

    for extension in extensions {
        ext_repo.add(&extension, lock, log).await?;
    }
    Ok(())
}

async fn run_extension_lock(args: &ArgMatches, conf: &Config) -> Result<(), anyhow::Error> {
    let ext_repo = ExtensionRepo::try_from(&conf.extensions)?;

    let lock = args.get_one::<bool>("lock").copied().unwrap_or_default();
    let log = &mut stdout();

    let extensions = args
        .get_many::<String>("image")
        .expect("clap to require")
        .map(&String::as_str)
        .map(Reference::try_from)
        .map(|f| f.context("parsing image path"))
        .collect::<Result<Vec<Reference>, anyhow::Error>>()?;

    for extension in extensions {
        ext_repo.version_lock(&extension, lock, log).await?;
    }
    Ok(())
}
