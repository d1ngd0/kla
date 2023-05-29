use clap::{arg, command, Arg, ArgAction, ArgMatches, Command};
use config::Config;
use config::FileFormat;
use kla::{Error, OptionalFile, RequestArgsBuilder};
use regex::Regex;

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conf = Config::builder()
        .add_source(OptionalFile::new("config.toml", FileFormat::Toml))
        .add_source(OptionalFile::new("/etc/kla/config.toml", FileFormat::Toml))
        .build()?;

    let m = command!()
        .subcommand_required(false)
        .subcommand(Command::new("version").about("Show the version of this application"))
        .subcommand(
            Command::new("environments")
            .about("Show the environments that are available to you.")
            .alias("envs")
            .arg(arg!(-r --regex <STATEMENT> "A regex statement").required(false).default_value(".*"))
        )
        .arg(arg!(-e --env <ENVIRONMENT> "The environment we will run the request against").required(false))
        .arg(arg!(-t --template <TEMPLATE> "The template to use when formating the output. prepending with @ will read a file."))
        .arg(arg!(-o --output <FILE> "The file to write the output into"))
        .arg(Arg::new("args").action(ArgAction::Append))
        .get_matches();

    match m.subcommand() {
        Some(("version", _)) => run_version(),
        Some(("environments", envs)) => run_environments(envs, &conf),
        _ => run_root(&m, &conf).await,
    }
}

fn run_environments(args: &ArgMatches, conf: &Config) -> Result<(), Error> {
    let r = Regex::new(args.get_one::<String>("regex").unwrap())?;

    conf.get_table("environments")?
        .iter()
        .filter(|(k, v)| {
            let v = format!("{v}");
            r.is_match(&k[..]) || r.is_match(&v[..])
        })
        .for_each(|(k, v)| println!("{k} = {v}"));
    Ok(())
}

fn run_version() -> Result<(), Error> {
    println!("Version: {:?}", VERSION.unwrap_or("Not specified"));
    Ok(())
}

async fn run_root(args: &ArgMatches, conf: &Config) -> Result<(), Error> {
    let mut reqb = RequestArgsBuilder::new()
        .args(
            args.get_many::<String>("args")
                .unwrap_or_default()
                .map(|v| v.to_owned())
                .collect::<Vec<_>>(),
        )?
        .template(args.get_one::<String>("template").map(|v| v.clone()));

    if let Some(env) = args.get_one::<String>("env") {
        reqb = reqb.environment(env, conf)?
    }

    if let Some(output) = args.get_one::<String>("output") {
        reqb = reqb.output(output.clone())
    }

    let req_args = reqb.build()?;
    kla::request(req_args).await?;

    Ok(())
}
