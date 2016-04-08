#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![deny(missing_docs)]

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
//!         Ok(result) =>
//!             println!("{:?}", result),
//!         Err(Error::Api(code, msg)) =>
//!             println!("API returned error {}: {}", code, msg),
//!         Err(err) =>
//!             println!("Other error: {:?}", err),
//!     };
//! }
//! ```

extern crate url;
extern crate hyper;
extern crate rustc_serialize;

use hyper::header::{ContentType, UserAgent};
use hyper::mime::{Mime, SubLevel, TopLevel};

use std::error::{self, Error as StdError};
use std::io;
use std::fmt;


use rustc_serialize::json;

fn user_agent() -> UserAgent {
    UserAgent("diffbot/rust".to_owned())
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
    /// Custom-built API with a specific name
    Custom(String),
}

impl API {
    fn get_str(&self) -> &str {
        match *self {
            API::Analyze => "analyze",
            API::Article => "article",
            API::Product => "product",
            API::Discussion => "discussion",
            API::Image => "image",
            API::Video => "video",
            API::Custom(ref name) => name.as_ref(),
        }
    }

    fn get_url_string(&self, version: u8) -> String {
        get_api_url_string(self.get_str(), version)
    }

    fn get_url(&self, version: u8) -> hyper::Url {
        get_api_url(self.get_str(), version)
    }
}

fn get_api_url_string(api: &str, version: u8) -> String {
    format!("https://api.diffbot.com/v{}/{}", version, api)
}

fn get_api_url(api: &str, version: u8) -> hyper::Url {
    hyper::Url::parse(&get_api_url_string(api, version)).unwrap()
}




/// Error occuring during a call.
#[derive(Debug)]
pub enum Error {
    /// The API returned an error.
    Api(u32, String),
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
            json::ParserError::SyntaxError(_, _, _) => Error::Json,
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
        match *self {
            Error::Api(_, ref msg) => msg,
            Error::Json => "invalid JSON",
            Error::Io(ref err) => err.description(),
            Error::Http(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Api(_, _) | Error::Json => None,
            Error::Io(ref err) => Some(err),
            Error::Http(ref err) => Some(err),
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
    version: u8,

    client: hyper::Client,
}

impl Diffbot {
    /// Returns a Diffbot client that uses the given token and version.
    ///
    /// Valid versions: `1`, `2`, `3`.
    pub fn new<S: ToString>(token: S, version: u8) -> Self {
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
    pub fn call_with_options<S: ToString>(&self, api: API, target_url: &str,
                                          options: &[(S, S)])
                                          -> DiffbotResult {
        let url = self.prepare_url(api, target_url, options);

        let builder = self.client.get(url).header(user_agent());
        Diffbot::process_request(builder)
    }

    /// List existing crawls.
    pub fn list_crawls(&self) -> DiffbotResult {
        let mut url = self.get_api_url("crawl");
        url.set_query_from_pairs(vec![("token", &self.token)]);
        let builder = self.client.get(url).header(user_agent());
        Diffbot::process_request(builder)
    }

    // Things in common between crawl and bulk
    fn do_crawl_bulk<S: AsRef<str>>(&self, api: &str,
                                    main_options: Vec<(&str, &str)>,
                                    extra_options: &[(S, S)])
                                    -> DiffbotResult {
        let mut body = url::form_urlencoded::serialize(main_options);
        body.push('&');
        body.push_str(&url::form_urlencoded::serialize(extra_options));

        let url = self.get_api_url(api);

        let content_type = ContentType(Mime(TopLevel::Application,
                                            SubLevel::WwwFormUrlEncoded,
                                            vec![]));
        let builder = self.client
                          .post(url)
                          .body(body.as_bytes())
                          .header(content_type)
                          .header(user_agent());
        Diffbot::process_request(builder)
    }

    /// Post an entire html body to the API, without extra options.
    ///
    /// See `call_with_options` for information on the arguments.
    ///
    /// `target_url` here is the URL the page would have.
    /// It doesn't have to be accessible, but will be used when resolving links.
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
    pub fn post_body(&self, api: API, target_url: &str, body: &[u8])
                     -> DiffbotResult {
        self.post_body_with_options::<String>(api, target_url, body, &[])
    }

    /// Posti an entire html body to the API.
    ///
    /// See `call_with_options` for information on the arguments.
    ///
    /// `target_url` here is the URL the page would have.
    /// It doesn't have to be accessible, but will be used when resolving links.
    pub fn post_body_with_options<S: ToString>(&self, api: API,
                                               target_url: &str, body: &[u8],
                                               options: &[(S, S)])
                                               -> DiffbotResult {
        let url = self.prepare_url(api, target_url, options);

        let content_type = ContentType(Mime(TopLevel::Text,
                                            SubLevel::Html,
                                            vec![]));
        let builder = self.client
                          .post(url)
                          .body(body)
                          .header(content_type)
                          .header(user_agent());
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
    pub fn search_with_options<S: ToString>(&self, col: &str, query: &str,
                                            options: &[(S, S)])
                                            -> DiffbotResult {
        let url = self.prepare_search_url(col, query, options);

        let builder = self.client.get(url).header(user_agent());
        Diffbot::process_request(builder)
    }

    fn get_api_url(&self, api: &str) -> hyper::Url {
        get_api_url(api, self.version)
    }

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

    fn prepare_search_url<S: ToString>(&self, col: &str, query: &str,
                                       options: &[(S, S)])
                                       -> hyper::Url {
        let mut params = Vec::<(String, String)>::new();
        params.push(("token".to_string(), self.token.clone()));
        params.push(("col".to_string(), col.to_string()));
        params.push(("query".to_string(), query.to_string()));
        for &(ref key, ref value) in options.iter() {
            params.push((key.to_string(), value.to_string()));
        }

        // We control the URL, it should always be valid.
        let mut url = self.get_api_url("search");
        url.set_query_from_pairs(&params);

        url
    }

    // Returns the diffbot URL for the given call
    fn prepare_url<S: ToString>(&self, api: API, target_url: &str,
                                options: &[(S, S)])
                                -> hyper::Url {

        let mut params = Vec::<(String, String)>::new();
        params.push(("token".to_string(), self.token.clone()));
        params.push(("url".to_string(), target_url.to_string()));
        for &(ref key, ref value) in options.iter() {
            params.push((key.to_string(), value.to_string()));
        }

        // We control the URL, it should always be valid.
        let mut url = api.get_url(self.version);
        url.set_query_from_pairs(&params);

        url
    }

    /// Starts a bulk job.
    ///
    /// Starts a bulk job called `name` on the given url list, using `api_url` on each.
    pub fn bulk<S: AsRef<str> + ::std::borrow::Borrow<str>>
        (&self, name: &str, api: API, urls: &[S])
         -> DiffbotResult {
        self.bulk_with_options(name, api, urls, &[])
    }

    /// Starts a bulk job with extra options.
    ///
    /// Give `options` a list of (key, value) pairs.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate diffbot;
    /// # use diffbot::*;
    /// # fn main() {
    /// # let diffbot = Diffbot::v3("token");
    /// # println!("{:?}",
    /// diffbot.bulk_with_options("my_bulk_job", API::Analyze,
    ///                          &["http://my.first.page.com",
    ///                            "https://my.second.page.com"],
    ///                          &[("repeat", "7.0"),
    ///                            ("notifyEmail", "me@example.com")])
    /// # );
    /// # }
    /// ```
    pub fn bulk_with_options<S: AsRef<str> + ::std::borrow::Borrow<str>>
        (&self, name: &str, api: API, urls: &[S], options: &[(S, S)])
         -> DiffbotResult {
        let joined = urls.join(" ");
        let api_url = api.get_url_string(self.version);

        self.do_crawl_bulk("bulk",
                           vec![("name", name),
                                ("token", &self.token),
                                ("apiUrl", &api_url),
                                ("urls", &joined)],
                           options)
    }

    /// Retrieves the result from a bulk job
    pub fn get_bulk(&self, name: &str) -> DiffbotResult {
        self.do_crawl_bulk::<&str>("bulk",
                                   vec![("token", &self.token),
                                        ("name", name),
                                        ("format", "json")],
                                   &[])
    }

    /// Starts a crawl job.
    pub fn crawl<S: AsRef<str> + ::std::borrow::Borrow<str>>
        (&self, name: &str, api: API, seeds: &[S])
         -> DiffbotResult {

        self.crawl_with_options(name, api, seeds, &[])
    }

    /// Starts a crawl job with extra options.
    ///
    /// Give `options` a list of (key, value) pairs.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate diffbot;
    /// # use diffbot::*;
    /// # fn main() {
    /// # let diffbot = Diffbot::v3("token");
    /// # println!("{:?}",
    /// diffbot.crawl_with_options("my_crawl_job", API::Analyze,
    ///                            &["http://my.first.page.com",
    ///                              "https://my.second.page.com"],
    ///                            &[("repeat", "7.0"),
    ///                              ("maxHops", "3")])
    /// # );
    /// # }
    /// ```
    pub fn crawl_with_options<S: AsRef<str> + ::std::borrow::Borrow<str>>
        (&self, name: &str, api: API, seeds: &[S], options: &[(S, S)])
         -> DiffbotResult {

        let api_url = api.get_url_string(self.version);
        let joined = seeds.join(" ");

        self.do_crawl_bulk("crawl",
                           vec![("name", name),
                                ("token", &self.token),
                                ("apiUrl", &api_url),
                                ("seeds", &joined)],
                           options)
    }

    /// Retrieves the result from a crawl job.
    pub fn get_crawl(&self, name: &str) -> DiffbotResult {
        // TODO: specify `num` parameter
        self.do_crawl_bulk::<&str>("crawl",
                                   vec![("token", &self.token),
                                        ("name", name),
                                        ("format", "json")],
                                   &[])
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
    println!("{:?}",
             diffbot.search_with_options("GLOBAL-INDEX",
                                         "site:techcrunch.com sortby:date",
                                         &[("num", "2")]));
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
    println!("{:?}",
             diffbot.call_with_options(API::Analyze,
                                       "http://diffbot.com",
                                       &[("fields", "links,meta")]));
}

#[test]
fn test_crawl() {
    let diffbot = Diffbot::v3("insert_your_token_here");

    println!("{:?}", diffbot.crawl("crawl", API::Analyze, &["http://mysite.com"]));

    println!("{:?}", diffbot.list_crawls());
}

#[test]
fn test_post() {
    // Use `cargo test -- --nocapture` to see the output
    let diffbot = Diffbot::v3("insert_your_token_here");
    let res = diffbot.post_body(API::Article,
                                "http://my.website.com",
                                br#"
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

#[test]
#[cfg(feature = "real_test")]
fn test_real_search() {
    let diffbot = Diffbot::v3(env!("TOKEN"));
    diffbot.search("GLOBAL-INDEX", "diffbot").unwrap();
}

#[test]
#[cfg(feature = "real_test")]
fn test_real_analyze() {
    let diffbot = Diffbot::v3(env!("TOKEN"));
    diffbot.call(API::Analyze, "http://diffbot.com").unwrap();
}

#[test]
#[cfg(feature = "real_test")]
fn test_real_crawl_list() {
    let diffbot = Diffbot::v3(env!("TOKEN"));
    diffbot.list_crawls().unwrap();
}
