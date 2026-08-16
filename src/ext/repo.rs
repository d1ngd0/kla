use flate2::read::GzDecoder;
use log::error;
use oci_client::{client::ClientConfig, secrets::RegistryAuth, Client, Reference};
use semver::Version;
use tar::Archive;

use crate::{
    config::{self, Config},
    AuthenticationBuilderRepo, Context as _, Error, ExtensionSet, KResult, EXTENSION_ROOT,
};
use std::{
    fs::{self},
    hash::{DefaultHasher, Hash, Hasher as _},
    io::Write,
    path::{Path, PathBuf},
};

use super::Extension;

pub struct ExtensionRepo {
    dir: PathBuf,
    auth: AuthenticationBuilderRepo,
}

impl TryFrom<&config::Extensions> for ExtensionRepo {
    type Error = Error;

    fn try_from(value: &config::Extensions) -> Result<Self, Self::Error> {
        let dir = value
            .dir
            .as_ref()
            .ok_or(Error::from("No extension directory specified!"))?;

        // TODO: remove this clone
        let auth = AuthenticationBuilderRepo::try_from(value.registries.clone())?;
        ExtensionRepo::new(dir, auth)
    }
}

impl ExtensionRepo {
    /// new creates a new repo extension from the following directory
    pub fn new<P: AsRef<Path>>(dir: P, auth: AuthenticationBuilderRepo) -> KResult<Self> {
        // make sure the directory exists, and return it
        if !dir.as_ref().exists() {
            fs::create_dir_all(dir.as_ref()).with_context(|| {
                format!(
                    "creating extension directory {}",
                    dir.as_ref().to_string_lossy()
                )
            })?;
        }

        Ok(Self {
            dir: PathBuf::from(dir.as_ref()),
            auth: auth,
        })
    }

    /// oci_client returns an opinionated client to the caller to fetch or push things
    fn oci_client(&self) -> KResult<Client> {
        Ok(Client::new(ClientConfig::default()))
    }

    /// oci_client_auth returns an opinionated authentication mechanism that caller should
    /// use
    fn oci_client_auth(&self, refr: &Reference) -> KResult<RegistryAuth> {
        let auth = self.auth.fetch(refr)?;
        Ok(auth)
    }

    /// extensions returns the current extensions from the extensions file
    pub fn extensions(&self) -> KResult<ExtensionSet> {
        let path = self.dir.join(EXTENSION_ROOT);
        if path.exists() {
            let extensions = fs::read_to_string(&path).with_context(|| {
                format!(
                    "reading extension file {}",
                    path.as_path().to_string_lossy()
                )
            })?;
            Ok(toml::from_str(&extensions).with_context(|| {
                format!(
                    "unmarshaling extension file {}",
                    path.as_path().to_string_lossy()
                )
            })?)
        } else {
            Ok(ExtensionSet::empty())
        }
    }

    /// commit_extension adds a single extension by loading the file,
    /// and writing the value.
    fn commit_extension(&self, ext: Extension) -> KResult<()> {
        let mut extensions = self.extensions()?;

        match extensions.iter().position(|f| f == &ext) {
            Some(i) => extensions[i] = ext,
            None => extensions.push(ext),
        }

        self.commit_extensions(extensions)
            .context("commiting extensions")
    }

    /// commit_extensions writes extensions back to the file
    fn commit_extensions(&self, extensions: ExtensionSet) -> KResult<()> {
        let path = self.dir.join(EXTENSION_ROOT);
        let extensions = toml::to_string(&extensions).context("marshaling extensions")?;
        Ok(fs::write(path.as_path(), &extensions).with_context(|| {
            format!(
                "writing extensions file {}",
                path.as_path().to_string_lossy()
            )
        })?)
    }

    /// remove removes an extension from kla
    pub fn remove(&self, image: &Reference) -> KResult<()> {
        let mut extensions = self.extensions()?;
        let removed = if let Some(index) = extensions.iter().position(|f| f == image) {
            extensions.remove(index)
        } else {
            return Err(Error::from(format!("extension {} not installed", image)));
        };

        // remove the extension
        self.commit_extensions(extensions)?;
        // clean up the templates
        fs::remove_dir_all(removed.dir)?;

        Ok(())
    }

    /// update reaches out to the registry of each extension and grabs the latest tags and updates
    /// to the newest version.
    pub async fn update<W: Write, R: AsRef<Reference>>(
        &self,
        image: R,
        stdout: &mut W,
    ) -> KResult<()> {
        let image = image.as_ref();

        let current = image
            .tag()
            .ok_or(Error::from("no version defined"))
            .and_then(|f| Version::parse(f).map_err(Error::from))
            .with_context(|| format!("currently installed: {}", image))?;
        let auth = self.oci_client_auth(image)?;
        let tags = self
            .oci_client()?
            .list_tags(image, &auth, None, None)
            .await?;

        let max = tags
            .tags
            .into_iter()
            .filter_map(|tag| Version::parse(tag.as_str()).ok())
            .max()
            .ok_or_else(|| Error::from("no semantic versions available"))?;

        if max > current {
            let updated = Reference::with_tag(
                image.registry().into(),
                image.repository().into(),
                max.to_string(),
            );

            let _ = writeln!(stdout, "upgrading from {} -> {}", image, updated);
            let _ = self.add(&updated, false, stdout).await?;
        }

        Ok(())
    }

    /// add installs the extension into the repo. If the extension already exists it will
    /// remove it before pulling the latest image and installing it. The extension name
    /// is defined by the registry and repository, so docker.example.com/ceph/admin:latest
    /// would be stored in <extension_directory>/docker.example.com/ceph/admin. Calling this
    /// function with a new `tag` would effectively update the extension.
    pub async fn add<W: Write>(
        &self,
        image: &Reference,
        lock: bool,
        stdout: &mut W,
    ) -> KResult<PathBuf> {
        let ext_dir = self.dir.join(image.registry()).join(image.repository());
        if ext_dir.exists() {
            fs::remove_dir_all(&ext_dir).with_context(|| {
                format!(
                    "removing previous extension directory {}",
                    ext_dir.as_path().to_string_lossy()
                )
            })?;
        }

        let _ = writeln!(stdout, "Pulling {}", image);
        let layers = self
            .oci_client()?
            .pull(
                &image,
                &self.oci_client_auth(image)?,
                vec!["application/vnd.oci.image.layer.v1.tar+gzip"],
            )
            .await?
            .layers
            .into_iter();

        for layer in layers {
            let mut hasher = DefaultHasher::new();
            layer.hash(&mut hasher);
            let hash = hasher.finish();
            let _ = writeln!(stdout, "unpacking layer <{:x}>", hash);
            let mut layer = Archive::new(GzDecoder::new(layer.data.as_ref()));
            layer
                .unpack(&ext_dir)
                .with_context(|| format!("merging layer {:x}", hash))?
        }

        let _ = write!(
            stdout,
            "installed extension to {}",
            &ext_dir.to_string_lossy()
        );

        self.commit_extension(Extension {
            dir: ext_dir.clone(),
            remote: image.clone(),
            lock: lock,
        })?;

        Ok(ext_dir)
    }

    pub async fn version_lock<W: Write>(
        &self,
        image: &Reference,
        enabled: bool,
        stdout: &mut W,
    ) -> KResult<()> {
        let mut extensions = self.extensions()?;
        let extension = if let Some(index) = extensions.iter().position(|f| f == image) {
            extensions
                .get_mut(index)
                .context("fetching extension from list after index")?
        } else {
            return Err(Error::from(format!("extension {} not installed", image)));
        };

        extension.lock = enabled;
        let extension_string = format!("{}", extension);

        self.commit_extensions(extensions)
            .with_context(|| format!("locking extension {}", extension_string))?;

        Ok(write!(stdout, "{}", extension_string)?)
    }

    pub fn apply(&self, config: &mut Config) -> KResult<()> {
        let extensions = self.extensions().context("fetching extensions")?;
        for extension in extensions.iter() {
            match Config::try_from(extension).with_context(|| {
                format!("loading extension configuration for {}", &extension.remote)
            }) {
                Ok(ext_config) => Some(ext_config),
                Err(err) => {
                    error!("{}: {}", extension.remote, err);
                    None
                }
            }
            .map(|ext_config| config.merge(ext_config));
        }
        Ok(())
    }
}
