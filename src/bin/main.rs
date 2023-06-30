use clap::{arg, command, Arg, ArgAction, ArgMatches, Command};
use config::Config;
use config::FileFormat;
use kla::{Error, KlaClient, KlaClientBuilder, KlaRequestBuilder, OptionalFile, TemplateBuilder};
use regex::Regex;
use reqwest::ClientBuilder;

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
        .arg(arg!(--agent <AGENT> "The header agent string").default_value("TODO: make it good"))
        .arg(arg!(-e --env <ENVIRONMENT> "The environment we will run the request against").required(false))
        .arg(arg!(-t --template <TEMPLATE> "The template to use when formating the output. prepending with @ will read a file."))
        .arg(arg!(-t --failure-template <TEMPLATE> "The template to use when formating the failure output. prepending with @ will read a file."))
        .arg(arg!(-o --output <FILE> "The file to write the output into"))
        .arg(arg!(--timeout <SECONDS> "The amount of time allotted for the request to finish"))
        .arg(arg!(--basic-auth <BASIC_AUTH> "The username and password seperated by :, a preceding @ denotes a file path."))
        .arg(arg!(--bearer-token <BEARER_TOKEN> "The bearer token to use in requests. A preceding @ denotes a file path."))
        .arg(arg!(-H --header <HEADER> "Specify a header The key and value should be seperated by a : (eg --header \"Content-Type: application/json\")").action(ArgAction::Append))
        .arg(arg!(-Q --query <QUERY> "Specify a query parameter The key and value should be seperated by a = (eg --query \"username=Jed\")").action(ArgAction::Append))
        .arg(arg!(-F --form <FORM> "Specify a form key=value to be passed in the form body").action(ArgAction::Append))
        .arg(arg!(-v --verbose "make it loud and proud").action(ArgAction::SetTrue))
        .arg(arg!(--dry "don't actually do anything, will automatically enable verbose").action(ArgAction::SetTrue))
        .arg(arg!(--http-version <HTTP_VERSION> "The version of http to send the request as").value_parser(["0.9", "1.0", "1.1", "2.0", "3.0"]))
        .arg(arg!(--no-gzip "Do not automatically uncompress gzip responses").action(ArgAction::SetTrue))
        .arg(arg!(--no-brotli "Do not automatically uncompress brotli responses").action(ArgAction::SetTrue))
        .arg(arg!(--no-deflate "Do not automatically uncompress deflate responses").action(ArgAction::SetTrue))
        .arg(arg!(--max-redirects <NUMBER> "The number of redirects allowed"))
        .arg(arg!(--no-redirects "Disable any redirects").action(ArgAction::SetTrue))
        .arg(arg!(--proxy <PROXY> "The proxy to use for all requests."))
        .arg(arg!(--proxy-http <PROXY_HTTP> "The proxy to use for http requests."))
        .arg(arg!(--proxy-https <PROXY_HTTPS> "The proxy to use for https requests."))
        .arg(arg!(--proxy-auth <PROXY_AUTH> "The username and password seperated by :."))
        .arg(arg!(--connect-timeout <DURATION> "The amount of time to allow for connection"))
        .arg(arg!(--certificate <CERTIFICATE_FILE> "The path to the certificate to use for requests. Accepts PEM and DER, expects files to end in .der or .pem. defaults to pem").action(ArgAction::Append))
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
    let env = kla::environment(args.get_one("env"), conf);

    TemplateBuilder::new_opt_file(args.get_one("output"))?
        .opt_template(args.get_one("template"))?
        .opt_failure_template(args.get_one("failure-template"))?
        .request(
            ClientBuilder::new()
                .opt_header_agent(args.get_one("agent"))?
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
                    args.get_one::<bool>("no_redirects")
                        .map(|v| *v)
                        .unwrap_or_default(),
                )
                .opt_proxy(args.get_one("proxy"), args.get_one("proxy-auth"))?
                .opt_proxy_http(args.get_one("proxy-http"), args.get_one("proxy-auth"))?
                .opt_proxy_https(args.get_one("proxy-https"), args.get_one("proxy-auth"))?
                .opt_certificate(args.get_many("certificate"))?
                .build()?
                .args(args.get_many("args"), env.as_ref())?
                .opt_headers(args.get_many("header"))?
                .opt_bearer_auth(args.get_one("bearer-token"))
                .opt_basic_auth(args.get_one("basic-auth"))
                .opt_query(args.get_many("query"))?
                .opt_form(args.get_many("form"))?
                .opt_timeout(args.get_one("timeout"))?
                .opt_version(args.get_one("http-version"))?,
        )
        .build()?
        .send()
        .await?;

    Ok(())
}
