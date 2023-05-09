use clap::Parser;
use krla::{Config, Error, RootArgs};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = RootArgs::parse();
    let conf = Config::new("config.toml")?;

    krla::run(args, conf)
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct RootArgs {
    arg1: Option<String>,

    arg2: Option<String>,

    arg3: Option<String>,

    #[arg(short, long)]
    env: Option<String>,

    #[arg(short, long)]
    template: Option<String>,
}

impl RootArgs {
    // url will return the url as supplied by the arguments
    fn url(&self, prefix: &str) -> Result<Url, ParseError> {
        let mut uri = "/";

        if let Some(arg) = self.arg2.as_ref() {
            uri = arg.as_str();
        } else if let Some(arg) = self.arg1.as_ref() {
            uri = arg.as_str();
        }

        let mut url = match prefix.strip_suffix("/") {
            Some(p) => String::from(p),
            None => String::from(prefix),
        };

        url.push_str(uri);

        Url::parse(url.as_str())
    }

    fn method(&self) -> Result<Method, InvalidMethod> {
        let method = if self.arg2.is_some() {
            self.arg1.as_ref().expect("how did you? what now?").as_str()
        } else {
            "GET"
        };

        Method::from_str(method.to_ascii_uppercase().as_str())
    }

    fn body(&self) -> Option<Body> {
        let body = self.arg3.as_ref()?;
        Some(Body::from(body.clone()))
    }

    fn env(&self) -> String {
        if let Some(s) = self.env.as_ref() {
            String::from(s)
        } else {
            String::from("")
        }
    }

    fn template(&self, name: &str) -> Result<Tera, tera::Error> {
        let mut tera = Tera::default();
        let template = match self.template.as_ref() {
            Some(v) => v.as_str(),
            None => "{{ body | json_encode(pretty=true) }}",
        };

        tera.add_raw_template(name, template)?;

        Ok(tera)
    }
}
