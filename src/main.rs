use clap::Parser;
use config::Config;
use config::FileFormat;
use kla::{Error, OptionalFile, RequestArgsBuilder};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = RootArgs::parse();
    let conf = Config::builder()
        .add_source(OptionalFile::new("config.toml", FileFormat::Toml))
        .add_source(OptionalFile::new("/etc/kla/config.toml", FileFormat::Toml))
        .build()?;

    let mut reqb = RequestArgsBuilder::new()
        .args(args.args)?
        .template(args.template);

    if let Some(env) = args.env {
        reqb = reqb.environment(&env, conf)?
    }

    let req_args = reqb.build()?;
    kla::request(req_args).await?;

    Ok(())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct RootArgs {
    args: Vec<String>,

    #[arg(short, long)]
    env: Option<String>,

    #[arg(short, long)]
    template: Option<String>,
}
