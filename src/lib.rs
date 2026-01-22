#![warn(clippy::pedantic)]
#![warn(missing_docs)]
//! Extra utilities for [`reqwest`](https://crates.io/crates/reqwest).

pub use reqwest;

use std::{error::Error, fmt::Display};

use bytes::Bytes;
use reqwest::Response;

/// A [`reqwest::Error`] that may also contain the response body.
///
/// Created from a response using the
/// [`ResponseExt::error_for_status_with_body`] method. Can also be created from
/// a [`reqwest::Error`].
///
/// # Example
///
/// ```
/// use reqwest_extra::{ErrorWithBody, ResponseExt};
///
/// async fn fetch_string(url: &str) -> Result<String, ErrorWithBody> {
///     let response = reqwest::get(url)
///         .await?
///         .error_for_status_with_body()
///         .await?
///         .text()
///         .await?;
///     Ok(response)
/// }
///
/// # #[tokio::main]
/// # async fn main() {
/// let err = fetch_string("https://api.github.com/user").await.unwrap_err();
/// println!("{err}");
/// # }
/// ```
///
/// Output (line-wrapped for readability):
/// ```text
/// HTTP status client error (403 Forbidden) for url (https://api.github.com/user),
/// body: b"\r\nRequest forbidden by administrative rules.
/// Please make sure your request has a User-Agent header
/// (https://docs.github.com/en/rest/overview/resources-in-the-rest-api#user-agent-required).
/// Check https://developer.github.com for other possible causes.\r\n"
/// ```
#[derive(Debug)]
pub struct ErrorWithBody {
    inner: reqwest::Error,
    body: Option<Result<Bytes, reqwest::Error>>,
}

impl ErrorWithBody {
    /// Get a reference to the inner [`reqwest::Error`].
    #[must_use]
    pub fn inner(&self) -> &reqwest::Error {
        &self.inner
    }

    /// Get a mutable reference to the inner [`reqwest::Error`].
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut reqwest::Error {
        &mut self.inner
    }

    /// Consume the `ErrorWithBody`, returning the inner [`reqwest::Error`].
    #[must_use]
    pub fn into_inner(self) -> reqwest::Error {
        self.inner
    }

    /// Get a reference to the response body, if available.
    #[must_use]
    pub fn body(&self) -> Option<&Result<Bytes, reqwest::Error>> {
        self.body.as_ref()
    }

    /// Get a mutable reference to the response body, if available.
    #[must_use]
    pub fn body_mut(&mut self) -> Option<&mut Result<Bytes, reqwest::Error>> {
        self.body.as_mut()
    }

    /// Consume the `ErrorWithBody`, returning the response body, if available.
    #[must_use]
    pub fn into_body(self) -> Option<Result<Bytes, reqwest::Error>> {
        self.body
    }

    /// Consume the `ErrorWithBody`, returning both the inner [`reqwest::Error`]
    /// and the response body, if available.
    #[must_use]
    pub fn into_parts(self) -> (reqwest::Error, Option<Result<Bytes, reqwest::Error>>) {
        (self.inner, self.body)
    }

    /// Add a url related to this error (overwriting any existing).
    #[must_use]
    pub fn with_url(self, url: reqwest::Url) -> Self {
        ErrorWithBody {
            inner: self.inner.with_url(url),
            body: self.body,
        }
    }

    /// Strip the related url from this error (if, for example, it contains
    /// sensitive information).
    #[must_use]
    pub fn without_url(self) -> Self {
        ErrorWithBody {
            inner: self.inner.without_url(),
            body: self.body,
        }
    }
}

impl Display for ErrorWithBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)?;
        if let Some(body) = &self.body {
            match body {
                Ok(body) => {
                    write!(f, ", body: {body:?}")?;
                }
                Err(body_error) => {
                    write!(f, ", error reading body: {body_error}")?;
                }
            }
        }
        Ok(())
    }
}

impl Error for ErrorWithBody {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner)
    }
}

impl From<reqwest::Error> for ErrorWithBody {
    fn from(err: reqwest::Error) -> Self {
        ErrorWithBody {
            inner: err,
            body: None,
        }
    }
}

/// Extension trait for [`reqwest::Response`] to provide additional
/// functionality.
pub trait ResponseExt: sealed::Sealed {
    /// Like [`reqwest::Response::error_for_status`], but if the response is an
    /// error, also reads and includes the response body in the returned
    /// error.
    ///
    /// # Example
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// use reqwest_extra::ResponseExt;
    ///
    /// let response = reqwest::get("https://api.github.com/user").await.unwrap();
    /// let err = response.error_for_status_with_body().await.unwrap_err();
    /// println!("{err}");
    /// # }
    /// ```
    ///
    /// Output (line-wrapped for readability):
    /// ```text
    /// HTTP status client error (403 Forbidden) for url (https://api.github.com/user),
    /// body: b"\r\nRequest forbidden by administrative rules.
    /// Please make sure your request has a User-Agent header
    /// (https://docs.github.com/en/rest/overview/resources-in-the-rest-api#user-agent-required).
    /// Check https://developer.github.com for other possible causes.\r\n"
    /// ```
    fn error_for_status_with_body(
        self,
    ) -> impl Future<Output = Result<Response, ErrorWithBody>> + Send + Sync + 'static;
}

impl ResponseExt for Response {
    async fn error_for_status_with_body(self) -> Result<Response, ErrorWithBody> {
        match self.error_for_status_ref() {
            Ok(_) => Ok(self),
            Err(e) => {
                let body = self.bytes().await;
                Err(ErrorWithBody {
                    inner: e,
                    body: Some(body),
                })
            }
        }
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for reqwest::Response {}
}
