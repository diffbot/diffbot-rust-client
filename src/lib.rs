//! This library provides an API client for [Diffbot](https://www.diffbot.com)
//!
//! See also [the diffbot documentation](https://www.diffbot.com/dev/docs/).
//!
//! # Example
//!
//! ```
//! extern crate diffbot;
//! use diffbot::*;
//!
//! fn main() {
//!     let client = Diffbot::v3("insert_your_token_here");
//!     match client.call(API::Analyze, "http://www.diffbot.com") {
//!         Ok(result) => println!("{:?}", result),
//!         Err(Error::Api(code, msg)) => println!("API returned error {}: {}", code, msg),
//!         Err(err) => println!("Other error: {:?}", err),
//!     };
//! }
//! ```

extern crate url;
extern crate hyper;
extern crate rustc_serialize;

use hyper::header::ContentType;
use hyper::mime::{Mime,TopLevel,SubLevel};

use std::error::{self, Error as StdError};
use std::io;
use std::fmt;


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

    client: hyper::Client,
}

/// One of the possible diffbot API.
///
/// See [the diffbot documentation](https://www.diffbot.com/dev/docs/).
pub enum API {
    /// The analyze API automatically detects the page type.
    Analyze,
    /// The article API for news article.
    Article,
    /// The product API for products in online shops.
    Product,
    /// The discussion API for forums.
    Discussion,
    /// The image API for image-central pages.
    Image,
    /// The video API for video pages (youtube, ...).
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
    /// The API returned an error.
    Api(u32,String),
    /// An error occured when decoding JSON from the API.
    Json,
    /// An error occured with the network.
    Io(io::Error),
    // TODO: don't expose hyper
    /// An HTTP error occured with the webserver.
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

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            &Error::Api(_, ref msg) => msg,
            &Error::Json => "invalid JSON",
            &Error::Io(ref err) => err.description(),
            &Error::Http(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            &Error::Api(_,_) => None,
            &Error::Json => None,
            &Error::Io(ref err) => Some(err),
            &Error::Http(ref err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.description())
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
            client: hyper::Client::new(),
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
    ///
    /// Just calls `call_with_options` with an empty option list.
    pub fn call(&self, api: API, target_url: &str) -> DiffbotResult {
        self.call_with_options::<String>(api, target_url, &[])
    }

    /// Makes an API call
    ///
    /// Runs `target_url` through the diffbot endpoint specified by `api`.
    /// Add each (key,value) pair in `options` to the query string.
    /// Read the [diffbot documentation](https://www.diffbot.com/dev/docs/)
    /// for information on supported values.
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
    pub fn call_with_options<S: ToString>(&self,
                                          api: API, target_url: &str,
                                          options: &[(S,S)]) -> DiffbotResult {
        let url = self.prepare_url(api, target_url, options);

        let builder = self.client.get(url);
        Diffbot::process_request(builder)
    }

    /// Posti an entire html body to the API, without extra options.
    ///
    /// See `call_with_options` for information on the arguments.
    ///
    /// `target_url` here is the URL the page would have. It doesn't have to be accessible, but
    /// will be used when resolving links.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate diffbot;
    /// # use diffbot::*;
    /// # fn main() {
    /// # let diffbot = Diffbot::v3("token");
    /// # println!("{:?}", {
    /// let body = b"<html>...</html>";
    /// diffbot.post_body(API::Article,
    ///                   "http://my.website.com",
    ///                   body)
    /// # } );
    /// # }
    /// ```
    pub fn post_body(&self, api: API, target_url: &str, body: &[u8]) -> DiffbotResult {
        self.post_body_with_options::<String>(api, target_url, body, &[])
    }

    /// Posti an entire html body to the API.
    ///
    /// See `call_with_options` for information on the arguments.
    ///
    /// `target_url` here is the URL the page would have.
    /// It doesn't have to be accessible, but will be used when resolving links.
    pub fn post_body_with_options<S: ToString>(&self,
                                               api: API, target_url: &str,
                                               body: &[u8],
                                               options: &[(S,S)]) -> DiffbotResult {
        let url = self.prepare_url(api, target_url, options);

        let header = ContentType(Mime(TopLevel::Text, SubLevel::Html, vec![]));
        let builder = self.client.post(url).body(body).header(header);
        Diffbot::process_request(builder)
    }

    /// Run a search in a diffbot collection without extra options.
    ///
    /// Use `col` = `GLOBAL-INDEX` for the global search collection.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate diffbot;
    /// # use diffbot::*;
    /// # fn main() {
    /// # let diffbot = Diffbot::v3("token");
    /// # println!("{:?}",
    /// diffbot.search("GLOBAL-INDEX", "diffbot")
    /// # );
    /// # }
    /// ```
    pub fn search(&self, col: &str, query: &str) -> DiffbotResult {
        self.search_with_options::<String>(col, query, &[])
    }

    /// Run a search in a diffbot collection.
    ///
    /// Use `col` = `GLOBAL-INDEX` for the global search collection.
    pub fn search_with_options<S: ToString>(&self,
                                            col: &str, query: &str,
                                            options: &[(S,S)]) -> DiffbotResult {
        let url = self.prepare_search_url(col, query, options);

        let builder = self.client.get(url);
        Diffbot::process_request(builder)
    }

    // TODO: add crawlbot API

    // Process a request and analyze the result
    fn process_request(builder: hyper::client::RequestBuilder) -> DiffbotResult {
        let mut result = try!(builder.send());

        let json_result = match try!(json::Json::from_reader(&mut result)) {
            json::Json::Object(obj) => obj,
            _ => return Err(Error::Json),
        };

        if json_result.contains_key("error") {
            let error_code = json_result.get("errorCode")
                .and_then(|c| c.as_u64())
                .unwrap_or(0u64);
            let error = json_result["error"].as_string().unwrap_or("");
            return Err(Error::Api(error_code as u32, error.to_string()));
        }

        Ok(json_result)
    }

    fn prepare_search_url<S: ToString>(&self,
                                       col: &str, query: &str,
                                       options: &[(S,S)]) -> hyper::Url {
        let mut params = Vec::<(String,String)>::new();
        params.push(("token".to_string(), self.token.clone()));
        params.push(("version".to_string(), self.version.to_string()));
        params.push(("col".to_string(), col.to_string()));
        params.push(("query".to_string(), query.to_string()));
        for &(ref key, ref value) in options.iter() {
            params.push((key.to_string(), value.to_string()));
        }

        // We control the URL, it should always be valid.
        let mut url = hyper::Url::parse("https://diffbot.com/api/search").unwrap();
        url.set_query_from_pairs(&params);

        url
    }

    // Returns the diffbot URL for the given call
    fn prepare_url<S: ToString>(&self,
                                api: API, target_url: &str,
                                options: &[(S,S)]) -> hyper::Url {

        let mut params = Vec::<(String,String)>::new();
        params.push(("token".to_string(), self.token.clone()));
        params.push(("version".to_string(), self.version.to_string()));
        params.push(("url".to_string(), target_url.to_string()));
        for &(ref key, ref value) in options.iter() {
            params.push((key.to_string(), value.to_string()));
        }

        // We control the URL, it should always be valid.
        let mut url = hyper::Url::parse(&api.get_url()).unwrap();
        url.set_query_from_pairs(&params);

        url
    }
}

#[test]
fn test_search() {
    let diffbot = Diffbot::v3("insert_your_token_here");
    println!("{:?}", diffbot.search("GLOBAL-INDEX", "diffbot"));
}

#[test]
fn test_search_with_options() {
    let diffbot = Diffbot::v3("insert_your_token_here");
    println!("{:?}", diffbot.search_with_options("GLOBAL-INDEX", "diffbot", &[("num","20")]));
}

#[test]
fn test_call() {
    // Use `cargo test -- --nocapture` to see the output
    let diffbot = Diffbot::v3("insert_your_token_here");
    println!("{:?}", diffbot.call(API::Analyze, "http://diffbot.com"));
}

#[test]
fn test_call_with_options() {
    // Use `cargo test -- --nocapture` to see the output
    let diffbot = Diffbot::v3("insert_your_token_here");
    println!("{:?}", diffbot.call_with_options(API::Analyze,
                                               "http://diffbot.com",
                                               &[("fields", "links,meta")]));
}

#[test]
fn test_post() {
    // Use `cargo test -- --nocapture` to see the output
    let diffbot = Diffbot::v3("insert_your_token_here");
    let res = diffbot.post_body(API::Article, "http://my.website.com", br#"
<html>
    <head>
        <title>My Website</title>
    </head>
    <body>
        <h1>My Website</h1>
        <p>This is a fake website, yet we will analyze its content.
           Isn't it interesting?</p>
    </body>
</html>"#);

    println!("{:?}", res);
}
