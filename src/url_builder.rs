use crate::Result;

/// URLBuilders take in a path and render a fully qualified URL.
/// The input will be coming from the user, so it may not be formatted
/// perfectly (trailing and preceeding slashes etc).
pub trait URLBuilder {
    fn build(&self, path: &str) -> Result<String>;
}

#[derive(Clone, Debug)]
/// PrefixURLBuilder takes a prefix, and a suffix (at build time) and
/// smashes them together. It **does** ensure to handle any trailing
/// or preceding slashes to build a decent url. Other than that it does
/// no validation to ensure the url string is a valid url
pub struct PrefixURLBuilder {
    prefix: String,
}

impl PrefixURLBuilder {
    // new create a new PrefixURLBuilder and normalize the prefix by
    // appending a trailing slash if one is not already present.
    fn new<S: Into<String>>(prefix: S) -> Self {
        let mut prefix = prefix.into();

        // normalize the prefix
        if !prefix.ends_with("/") {
            prefix.push_str("/");
        };

        Self { prefix: prefix }
    }
}

impl URLBuilder for PrefixURLBuilder {
    // build creates the url by appending the path onto the prefix. It
    // also ensures the uri does not have any preceeding forward slashes
    // since the prefix will have one.
    fn build(&self, path: &str) -> Result<String> {
        let mut url = self.prefix.clone();
        url.push_str(path.trim_start_matches("/"));
        Ok(url)
    }
}

// Anything that can become a string can become a PrefixURLBuilder
impl<T: Into<String>> From<T> for PrefixURLBuilder {
    fn from(value: T) -> Self {
        PrefixURLBuilder::new(value)
    }
}

#[derive(Default, Clone, Copy)]
/// LiteralURLBuilder does nothing to the prefix, and assumes it is a
/// fully qualified path already.
struct LiteralURLBuilder {}

impl URLBuilder for LiteralURLBuilder {
    /// build just returns the path as a new string, it assumes the path
    /// given is a fully qualified domain.
    fn build(&self, path: &str) -> Result<String> {
        Ok(path.into())
    }
}

#[derive(Clone, Debug)]
/// AssumingURLBuilder will assume the path is a literal URL when it begins
/// with http: or https:, otherwise it runs it through an internal prefixedURL
/// Builder
pub struct AssumingURLBuilder {
    prefixed: PrefixURLBuilder,
}

impl AssumingURLBuilder {
    /// new returns a new AssumingURLBuilder which assumes the path is literal
    /// when it starts with http or https
    pub fn new<S: Into<String>>(prefix: S) -> Self {
        Self {
            prefixed: PrefixURLBuilder::new(prefix),
        }
    }
}

impl URLBuilder for AssumingURLBuilder {
    /// build looks at the path, if it starts with http:// or https:// we assume
    /// the path is literal, and return that. If not we use a prefix builder
    fn build(&self, path: &str) -> Result<String> {
        if path.starts_with("http://") || path.starts_with("https://") {
            return LiteralURLBuilder::default().build(path);
        } else {
            self.prefixed.build(path)
        }
    }
}

// implement an assuming builder for anything that can become a String
impl<T: Into<String>> From<T> for AssumingURLBuilder {
    fn from(value: T) -> Self {
        AssumingURLBuilder::new(value)
    }
}

// enbale turning a PrefixURLBuilder into an AssumingURLBuilder
impl From<PrefixURLBuilder> for AssumingURLBuilder {
    fn from(value: PrefixURLBuilder) -> Self {
        AssumingURLBuilder { prefixed: value }
    }
}

/// OptBaseURLBuilder creates a builder that when no base exists we call
/// a LiteralURLBuilder. When there is a base we call an AssumingURLBuilder
pub enum OptBaseURLBuilder {
    Empty,
    Base(AssumingURLBuilder),
}

impl OptBaseURLBuilder {
    /// empty is shorthand for creating an empty OptBaseUrlBuilder
    pub fn empty() -> Self {
        Self::Empty
    }

    /// new creates a new OptBaseURLBuilder with a base
    pub fn new<S: Into<AssumingURLBuilder>>(prefix: S) -> Self {
        Self::Base(prefix.into())
    }

    /// some_new creates a new OptBaseURLBuilder with a base of Some(val)
    /// and empty when None
    pub fn some_new<S: Into<AssumingURLBuilder>>(prefix: Option<S>) -> Self {
        match prefix {
            Some(base) => OptBaseURLBuilder::Base(base.into()),
            None => OptBaseURLBuilder::Empty,
        }
    }
}

impl URLBuilder for OptBaseURLBuilder {
    /// build will call the underlying AssumingURLBuilder when we have a base, and will
    /// call a LiteralURLBuilder when there is no path
    fn build(&self, path: &str) -> Result<String> {
        match self {
            OptBaseURLBuilder::Empty => LiteralURLBuilder::default().build(path),
            OptBaseURLBuilder::Base(assuming_urlbuilder) => assuming_urlbuilder.build(path),
        }
    }
}

// Anything that can become a string can become a OptBaseUrlBuilder
// with a base
impl<T: Into<String>> From<T> for OptBaseURLBuilder {
    fn from(value: T) -> Self {
        OptBaseURLBuilder::new(value)
    }
}

// enable converting an AssumingURLBuilder into a OptBaseUrlBuilder
impl From<AssumingURLBuilder> for OptBaseURLBuilder {
    fn from(value: AssumingURLBuilder) -> Self {
        OptBaseURLBuilder::Base(value)
    }
}
