use clap::Parser;
use config::Config;
use config::{File, FileFormat};
use krla::{Error, RequestArgsBuilder};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = RootArgs::parse();
    let config = Config::builder()
        .add_source(File::new("config.toml", FileFormat::Toml))
        .build();

    let mut reqb = RequestArgsBuilder::new()
        .args(args.args)?
        .template(args.template);

    if let Some(env) = args.env {
        if let Ok(config) = config {
            reqb = reqb.environment(&env, config)?
        }
    }

    let req_args = reqb.build()?;
    krla::request(req_args).await?;

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
