use config::{ConfigError, File, FileFormat, FileSourceFile, FileStoredFormat, Map, Source, Value};
use std::fmt::Debug;
use std::path::Path;

#[derive(Debug)]
pub struct OptionalFile<F: FileStoredFormat + 'static>(Option<File<FileSourceFile, F>>);

impl<F> OptionalFile<F>
where
    F: FileStoredFormat + 'static,
{
    pub fn new(path: &str, format: F) -> OptionalFile<F> {
        if !Path::new(path).exists() {
            return OptionalFile(None);
        }

        OptionalFile(Some(File::new(path, format)))
    }

    pub fn with_name(path: &str) -> OptionalFile<FileFormat> {
        if !Path::new(path).exists() {
            return OptionalFile(None);
        }

        OptionalFile(Some(File::with_name(path)))
    }
}

impl<F> Source for OptionalFile<F>
where
    F: FileStoredFormat + Debug + Clone + Send + Sync + 'static,
{
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        match self.0.as_ref() {
            Some(file) => file.clone_into_box(),
            None => Box::new(OptionalFile::<F>(None)),
        }
    }

    fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        match self.0.as_ref() {
            Some(file) => file.collect(),
            None => Ok(Map::new()),
        }
    }

    fn collect_to(&self, cache: &mut Value) -> Result<(), ConfigError> {
        match self.0.as_ref() {
            Some(file) => file.collect_to(cache),
            None => Ok(()),
        }
    }
}
