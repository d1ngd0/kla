use clap::{arg, command, Arg, ArgAction, ArgMatches, Command};
use config::Config;
use config::FileFormat;
use kla::{Error, OptionalFile, RequestArgsBuilder};
use regex::Regex;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conf = Config::builder()
        .add_source(OptionalFile::new("config.toml", FileFormat::Toml))
        .add_source(OptionalFile::new("/etc/kla/config.toml", FileFormat::Toml))
        .build()?;

    let m = command!()
        .subcommand_required(false)
        .subcommand(
            Command::new("environments")
            .about("Show the environments that are available to you.")
            .alias("envs")
            .arg(arg!(-r --regex <STATEMENT> "A regex statement").required(false).default_value(".*"))
        )
        .arg(arg!(-e --env <ENVIRONMENT> "The environment we will run the request against").required(false))
        .arg(arg!(-t --template <TEMPLATE> "The template to use when formating the output. prepending with @ will read a file."))
        .arg(arg!(-o --output <FILE> "The file to write the output into"))
        .arg(arg!(--basic-auth <BASIC_AUTH> "The username and password seperated by :, a preceding @ denotes a file path."))
        .arg(arg!(--bearer-token <BEARER_TOKEN> "The bearer token to use in requests. A preceding @ denotes a file path."))
        .arg(arg!(-H --header <HEADER> "Specify a header The key and value should be seperated by a : (eg --header \"Content-Type: application/json\")").action(ArgAction::Append))
        .arg(arg!(-v --verbose "make it loud and proud").action(ArgAction::SetTrue))
        .arg(arg!(--dry "don't actually do anything, will automatically enable verbose").action(ArgAction::SetTrue))
        .arg(Arg::new("args").action(ArgAction::Append))
        .get_matches();

    match m.subcommand() {
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

async fn run_root(args: &ArgMatches, conf: &Config) -> Result<(), Error> {
    let req_args = RequestArgsBuilder::new()
        .args(
            args.get_many::<String>("args")
                .unwrap_or_default()
                .map(|v| v.to_owned())
                .collect::<Vec<_>>(),
        )?
        .template(args.get_one::<String>("template").map(|v| v.clone()))
        .environment(args.get_one("env"), conf)?
        .headers(args.get_many("header"))
        .output(args.get_one("output"))
        .bearer_token(args.get_one("bearer-token"))
        .basic_auth(args.get_one("basic-auth"))
        .verbose(args.get_one("verbose"))
        .dry(args.get_one("dry"))
        .build()?;

    kla::request(req_args).await?;

    Ok(())
}
