//! This library provides an API client for [Diffbot](https://www.diffbot.com)
//!
//! See also [the diffbot documentation](https://www.diffbot.com/dev/docs/).

extern crate url;
extern crate hyper;
extern crate rustc_serialize;

use std::io;

use rustc_serialize::json;

/// Diffbot API client.
///
/// # Example
///
/// ```
/// # extern crate diffbot;
/// # use diffbot::*;
/// # fn main() {
/// let diffbot = Diffbot::v3("token");
/// let result = diffbot.call(API::Analyze, "http://diffbot.com");
/// # println!("{:?}", result);
/// # }
/// ```
pub struct Diffbot {
    token: String,
    version: u32,
}

/// One of the possible diffbot API
pub enum API {
    Analyze,
    Article,
    Product,
    Discussion,
    Image,
    Video,
}

impl API {
    fn get_str(&self) -> &'static str {
        match self {
            &API::Analyze => "analyze",
            &API::Article => "article",
            &API::Product => "product",
            &API::Discussion => "discussion",
            &API::Image => "image",
            &API::Video => "video",
        }
    }

    fn get_url(&self) -> String {
        format!("https://diffbot.com/api/{}", self.get_str())
    }
}

/// Error occuring during a call.
#[derive(Debug)]
pub enum Error {
    Api(u32,String),
    Json,
    Io(io::Error),
    Http(hyper::Error),
}

impl From<json::ParserError> for Error {
    fn from(err: json::ParserError) -> Self {
        match err {
            json::ParserError::SyntaxError(_,_,_) => Error::Json,
            json::ParserError::IoError(err) => Error::Io(err),
        }
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::Http(err)
    }
}

/// Result from a call.
pub type DiffbotResult = Result<json::Object, Error>;

impl Diffbot {
    /// Returns a Diffbot client that uses the given token and version.
    ///
    /// Valid versions: `1`, `2`, `3`.
    pub fn new<S: ToString>(token: S, version: u32) -> Self {
        Diffbot {
            token: token.to_string(),
            version: version,
        }
    }

    /// Convenient method to use a v1 client.
    pub fn v1<S: ToString>(token: S) -> Self {
        Diffbot::new(token, 1)
    }

    /// Convenient method to use a v2 client.
    pub fn v2<S: ToString>(token: S) -> Self {
        Diffbot::new(token, 2)
    }

    /// Convenient method to use a v3 client (recommended).
    pub fn v3<S: ToString>(token: S) -> Self {
        Diffbot::new(token, 3)
    }

    /// Makes an API call without extra options.
    pub fn call(&self, api: API, url: &str) -> DiffbotResult {
        call(&self.token, self.version, api, url)
    }

    /// Makes an API call
    ///
    /// Add each (key,value) pair in `options` to the query string.
    /// Read the [diffbot documentation](https://www.diffbot.com/dev/docs/) for information on supported values.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate diffbot;
    /// # use diffbot::*;
    /// # fn main() {
    /// # let diffbot = Diffbot::v3("token");
    /// # println!("{:?}",
    /// diffbot.call_with_options(API::Article,
    ///                           "http://diffbot.com",
    ///                           &[("paging", "false")])
    /// # );
    /// # }
    /// ```
    pub fn call_with_options<S: ToString>(&self, api: API, url: &str, options: &[(S,S)]) -> DiffbotResult {
        call_with_options(&self.token, self.version, api, url, options)
    }
}

/// Makes an API call without extra options.
///
/// Just calls `call_with_options` with an empty option list.
pub fn call(token: &str, version: u32, api: API, target_url: &str) -> DiffbotResult {
    call_with_options::<String>(token, version, api, target_url, &[])
}

/// Makes an API call.
///
/// Runs `target_url` through the diffbot endpoint specified by `api`.
/// Adds every (key,value) pair from `options` to the query string.
pub fn call_with_options<S: ToString>(token: &str, version: u32, api: API, target_url: &str, options: &[(S,S)]) -> DiffbotResult {
    let mut params = Vec::<(String,String)>::new();
    params.push(("token".to_string(), token.to_string()));
    params.push(("version".to_string(), version.to_string()));
    params.push(("url".to_string(), target_url.to_string()));
    for &(ref key, ref value) in options.iter() {
        params.push((key.to_string(), value.to_string()));
    }

    let client = hyper::Client::new();

    // We control the URL, it should always be valid.
    let mut url = hyper::Url::parse(&api.get_url()).unwrap();
    url.set_query_from_pairs(&params);

    println!("{}", url);

    let builder = client.get(url);
    let mut result = try!(builder.send());

    let json_result = match try!(json::Json::from_reader(&mut result)) {
        json::Json::Object(obj) => obj,
        _ => return Err(Error::Json),
    };

    if json_result.contains_key("error") {
        let error_code = json_result.get("errorCode").and_then(|c| c.as_u64()).unwrap_or(0u64) as u32;
        let error = json_result["error"].as_string().unwrap_or("");
        return Err(Error::Api(error_code, error.to_string()));
    }

    Ok(json_result)
}

#[test]
fn test_call() {
    // Use `cargo test -- --nocapture` to see the output
    println!("{:?}", call("insert_your_token_here", 3,
                          API::Analyze,
                          "http://diffbot.com"));
}

#[test]
fn test_call_with_options() {
    // Use `cargo test -- --nocapture` to see the output
    println!("{:?}", call_with_options("insert_your_token_here", 3,
                                       API::Analyze,
                                       "http://diffbot.com",
                                       &[("fields", "links,meta")]));
}
