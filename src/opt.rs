use std::future::Future;

pub trait Opt: Sized {
    /// Error is the error which will be returned by with_some_result
    type Error;

    /// with_some_result takes a closure which returns a Result of self
    fn with_some_result<T, F>(self, v: Option<T>, f: F) -> Result<Self, Self::Error>
    where
        F: Fn(Self, T) -> Result<Self, Self::Error>;
    /// with_some allows you to send an optional value to the type which will only be
    /// called when v is Some. An Example usecase is
    ///
    /// command!()
    ///   .with_some(Some("this is a description"), Command::about);
    ///
    /// here the `about` command is only being called when there is a value, otherwise
    /// it just returns `self`
    fn with_some<T, F>(self, v: Option<T>, f: F) -> Self
    where
        F: Fn(Self, T) -> Self;
}

pub trait When: Sized {
    /// when calls the underlying builder function if v is true
    fn when<F>(self, v: bool, f: F) -> Self
    where
        F: Fn(Self) -> Self;
}

pub trait Ok: Sized {
    /// Error is the type of error value shared between the result coming from the underlying
    /// closure, and the incoming value
    type Error;

    /// with_ok works the same as `with_some` but for the error case. The error can come
    /// from either the value or the resulting function, but the errors must be the same
    /// type. The underlying function is only called when v is `Ok`.
    ///
    /// If you have something that returns a result, but you don't need the result and just
    /// want to skip setting that value, use `.ok` to turn the result into an option instead
    /// of using this function.
    fn with_ok<T, F>(self, v: Result<T, Self::Error>, f: F) -> Result<Self, Self::Error>
    where
        F: Fn(Self, T) -> Result<Self, Self::Error>;

    /// with_ok_value works the same as with_ok, but only the value has a result. The
    /// result of the value will bubble out to the call.
    ///
    /// example:
    /// ```compile_fail
    /// command!().with_ok_value(Ok("something"), Command::about);
    /// ```
    fn with_ok_value<T, F>(self, v: Result<T, Self::Error>, f: F) -> Result<Self, Self::Error>
    where
        F: Fn(Self, T) -> Self,
    {
        self.with_ok(v, |s, v| Ok(f(s, v)))
    }
}

#[macro_export]
macro_rules! impl_opt {
    ($for:ty, $error:ty) => {
        impl crate::opt::Opt for $for {
            /// Error specifies the error type when calling with_some_result
            type Error = $error;

            /// with_some_result allows you to send an optional value to the type which will only be
            /// called when v is Some. The closure itself then returns a result of self. An Example
            /// usecase is:
            ///
            /// command!()
            ///   .with_some(Some(&config.attr), ClientBuilder.with_attributes);
            ///
            /// here `with_attributes` returns a `Result<ClientBuilder>`
            fn with_some_result<T, F>(
                self,
                v: Option<T>,
                f: F,
            ) -> std::result::Result<Self, Self::Error>
            where
                F: Fn(Self, T) -> std::result::Result<Self, Self::Error>,
            {
                match v {
                    Some(v) => f(self, v),
                    None => Ok(self),
                }
            }

            /// with_some allows you to send an optional value to the type which will only be
            /// called when v is Some. An Example usecase is
            ///
            /// command!()
            ///   .with_some(Some("this is a description"), Command::about);
            ///
            /// here the `about` command is only being called when there is a value, otherwise
            /// it just returns `self`
            fn with_some<T, F>(self, v: Option<T>, f: F) -> Self
            where
                F: Fn(Self, T) -> Self,
            {
                match v {
                    Some(v) => f(self, v),
                    None => self,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_when {
    ($for:ty) => {
        impl crate::opt::When for $for {
            fn when<F>(self, v: bool, f: F) -> Self
            where
                F: Fn(Self) -> Self,
            {
                match v {
                    true => f(self),
                    false => self,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_ok {
    ($for:ty, $error:ty) => {
        impl crate::opt::Ok for $for {
            /// Error is the type of error value shared between the result coming from the underlying
            /// closure, and the incoming value
            type Error = $error;

            /// with_ok works the same as `with_some` but for the error case. The error can come
            /// from either the value or the resulting function, but the errors must be the same
            /// type. The underlying function is only called when v is `Ok`.
            ///
            /// If you have something that returns a result, but you don't need the result and just
            /// want to skip setting that value, use `.ok` to turn the result into an option instead
            /// of using this function.
            fn with_ok<T, F>(
                self,
                v: std::result::Result<T, Self::Error>,
                f: F,
            ) -> std::result::Result<Self, Self::Error>
            where
                F: Fn(Self, T) -> std::result::Result<Self, Self::Error>,
            {
                match v {
                    Ok(v) => f(self, v),
                    Err(err) => Err(err),
                }
            }
        }
    };
}

// Enables you to use async_or_else, so an async function that gives you an or_else
// function
pub trait AsyncOption {
    type Output;

    #[allow(async_fn_in_trait)]
    async fn async_or_else<F, Fut>(self, f: F) -> Option<Self::Output>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<Self::Output>>;
}

impl<T> AsyncOption for Option<T> {
    type Output = T;

    async fn async_or_else<F, Fut>(self, f: F) -> Option<Self::Output>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<Self::Output>>,
    {
        match self {
            x @ Some(_) => x,
            None => f().await,
        }
    }
}
