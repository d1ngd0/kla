use config::{File, FileFormat};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Config error")]
    ConfigError(String),
}

impl std::convert::From<config::ConfigError> for Error {
    fn from(err: config::ConfigError) -> Self {
        Error::ConfigError(err.to_string())
    }
}

pub struct Config {
    config: config::Config,
}

impl Config {
    pub fn new(file: &str) -> Result<Config, Error> {
        let config = config::Config::builder()
            .set_default("env", "")?
            .add_source(File::new(file, FileFormat::Toml))
            .build()?;

        Ok(Config { config })
    }

    pub fn prefix(&self, env: &str) -> String {
        let mut env_s = String::from(env);
        env_s.insert_str(0, "environments.");

        self.config
            .get_string(&env_s[..])
            .unwrap_or(String::from(""))
    }
}
