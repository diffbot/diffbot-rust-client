/*!
 * This library provides an API client for Diffbot.
 *
 * Making API requests
 * -------------------
 *
 * There are a handful of different ways to make API calls:
 *
 * 1. The most basic way to make a request is with the ``call()`` function.
 *    Everything must be specified for each request.
 *
 * 2. Use the ``Diffbot`` struct to keep track of your token and API version
 *    and then use its ``.call()`` method to make API calls. This has the
 *    advantage that you can specify those things just once and they'll be
 *    retained.
 *
 * 3. Instead of making a request in one step, you can make it two steps with
 *    the ``prepare_request()`` function. This allows you to specify to Diffbot
 *    certain details of how *it* should make the request. That gives you a
 *    ``Request`` object.
 *
 * 4. In the same manner, if you have a ``Diffbot`` struct you can call the
 *    ``.prepare_request()`` method on it.
 *
 * Prepared requests
 * -----------------
 *
 * If you use the ``prepare_request()`` function or method, you can tweak the
 * request that will be sent to Diffbot. You can alter the User-Agent, Referer
 * or Cookie headers that it will send and then call ``.call()`` to make the
 * request, or you can call ``.post_body()`` to send the HTML yourself, if it
 * is not publicly available to the wider Internet.
 *
 * Getting data out of the result
 * ------------------------------
 *
 * At present, the successful return value of a request is simply a JSON object,
 * a tree map. This *will* make it moderately difficult to work with, but if
 * you're determined, it's possible. You'll end up with results like these:
 *
 *     // First of all, you must, of course, have a response to work on.
 *     let mut response: TreeMap<~str, Json>
 *                     = diffbot::call(..., "article", ...).unwrap();
 *
 *     // Get the title of the article
 *     let title = match response.pop(&~"title").unwrap() {
 *         json::String(s) => s,
 *         _ => unreachable!(),
 *     };
 *
 *     // Get the URL of each image
 *     let image_urls: ~[Url] = match response.pop(&~"images").unwrap() {
 *         json::List(images) => images.move_iter().map(|image| match image {
 *             json::Object(~mut o) => {
 *                 match o.pop(&~"url").unwrap() {
 *                     json::String(ref s) => from_str(s),
 *                     _ => unreachable!(),
 *                 }
 *             },
 *             _ => unreachable!(),
 *         }),
 *         _ => unreachable!(),
 *     }.collect();
 *
 * (Yep, I'll freely admit that these are clumsier than they might be in another
 * language, which might allow something like this:
 *
 *     let response = ...;
 *
 *     let title = response.title;
 *     let image_urls = [from_str(image.url) for image in response.images];
 *
 * In time we may get strongly typed interfaces which would be much nicer, but
 * for now, you'd need to do that yourself. It can be done with the tools in
 * ``extra::serialize``, by the way.)
 */
#[crate_id = "diffbot#1.0"];
#[crate_type = "dylib"];
#[crate_type = "rlib"];
#[doc(html_logo_url = "diffy-d.png",
      html_favicon_url = "http://www.diffbot.com/favicon.ico")];

extern mod extra = "extra#0.10-pre";
extern mod http = "http#0.1-pre";

use std::io::net::tcp::TcpStream;
use extra::json;
use extra::url::Url;
use http::client::RequestWriter;
use http::method::{Get, Post};
use http::headers::content_type::MediaType;

/// A convenience type which simply keeps track of a developer token and version
/// number.
///
/// There is no necessity to use this type; you can call ``call()`` directly
/// should you so desire.
#[deriving(Eq, Clone)]
pub struct Diffbot {
    /// The developer's token
    token: ~str,

    /// The API version number
    version: uint,
}

// Basic methods
impl Diffbot {
    /// Construct a new ``Diffbot`` instance from the passed parameters.
    pub fn new(token: ~str, version: uint) -> Diffbot {
        Diffbot {
            token: token,
            version: version,
        }
    }

    /// Make a call to any Diffbot API with the stored token and API version.
    ///
    /// See the ``call()`` function for an explanation of the parameters.
    pub fn call(&self, url: &Url, api: &str, fields: &[&str])
               -> Result<json::Object, Error> {
        call(url, self.token, api, fields, self.version)
    }

    /// Prepare a request to any Diffbot API with the stored token and API version.
    ///
    /// See the ``call()`` function for an explanation of the parameters.
    pub fn prepare_request(&self, url: &Url, api: &str, fields: &[&str])
               -> Request {
        prepare_request(url, self.token, api, fields, self.version)
    }
}

/// An in-progress Diffbot API call.
pub struct Request {
    priv request: RequestWriter<TcpStream>,
}

impl Request {
    /// Set the value for Diffbot to send as the ``User-Agent`` header when
    /// making your request.
    pub fn user_agent(&mut self, user_agent: ~str) {
        self.request.headers.extensions.insert(~"X-Forwarded-User-Agent",
                                               user_agent);
    }

    /// Set the value for Diffbot to send as the ``Referer`` header when
    /// making your request.
    pub fn referer(&mut self, referer: ~str) {
        self.request.headers.extensions.insert(~"X-Forwarded-Referer",
                                               referer);
    }

    /// Set the value for Diffbot to send as the ``Cookie`` header when
    /// making your request.
    pub fn cookie(&mut self, cookie: ~str) {
        self.request.headers.extensions.insert(~"X-Forwarded-Cookie",
                                               cookie);
    }

    /// Set Diffbot's timeout, in milliseconds. The default is five seconds.
    pub fn timeout(&mut self, milliseconds: u64) {
        self.request.url.query.push((~"timeout", milliseconds.to_str()));
    }

    /// Execute the request and get the results.
    pub fn call(self) -> Result<json::Object, Error> {
        let mut response = match self.request.read_response() {
            Ok(response) => response,
            Err(_request) => return Err(IoError),  // Request failed
        };
        let json = match json::from_reader(&mut response as &mut Reader) {
            Ok(json) => json,
            Err(error) => return Err(JsonError(error)),  // It... wasn't JSON!?
        };
        // Now let's see if this is an API error or not.
        // API errors are of the form {"error":"Invalid API.","errorCode":500}
        match json {
            json::Object(~mut o) => {
                match o.pop(&~"errorCode") {
                    Some(json::Number(num)) => {
                        let num = num as uint;
                        let msg = match o.pop(&~"error")
                               .expect("JSON had errorCode but not error") {
                            json::String(s) => s,
                            uh_oh => fail!("error was {} instead of a string", uh_oh.to_str()),
                        };
                        Err(ApiError(msg, num))
                    },
                    Some(uh_oh) => fail!("errorCode was {} instead of a number", uh_oh.to_str()),
                    None => Ok(o),
                }
            },
            // All API responses must be objects.
            // If it's not, there's something screwy going on.
            _ => fail!("API return value wasn't a JSON object"),
        }
    }

    /// Execute the request as a POST request, sending it through with the given
    /// text/html entity body.
    ///
    /// This has the effect that Diffbot will skip requesting the URL and will
    /// instead take the passed body as the HTML it is to check. This is mainly
    /// useful for non-public websites.
    pub fn post_body(mut self, body: &[u8]) -> Result<json::Object, Error> {
        self.request.method = Post;
        self.request.headers.content_type = Some(MediaType(~"text", ~"html", ~[]));
        self.request.headers.content_length = Some(body.len());
        // Calling write_headers is an extra and unnecessary safety guard which
        // will cause the task to fail if the request has already started to be
        // sent (which would render the three statements above ineffectual)
        self.request.write_headers();
        self.request.write(body);
        self.call()
    }
}

/// Error code: "unauthorized token"
pub static UNAUTHORIZED_TOKEN: uint = 401;

/// Error code: "requested page not found"
pub static REQUESTED_PAGE_NOT_FOUND: uint = 404;

/// Error code: "your token has exceeded the allowed number of calls, or has
/// otherwise been throttled for API abuse."
pub static TOKEN_EXCEEDED_OR_THROTTLED: uint = 429;

/// Error code: "error processing the page. Specific information will be
/// returned in the JSON response."
pub static ERROR_PROCESSING: uint = 500;

/// Something went wrong with the Diffbot API call.
#[deriving(Eq)]
pub enum Error {
    /// An error code returned by the Diffbot API, with message and code.
    /// Refer to http://www.diffbot.com/dev/docs/error/ for an explanation of
    /// the error codes.
    ///
    /// When comparing the error code, you should use these constants:
    ///
    /// - ``UNAUTHORIZED_TOKEN``: "unauthorized token"
    /// - ``REQUESTED_PAGE_NOT_FOUND``: "requested page not found"
    /// - ``TOKEN_EXCEEDED_OR_THROTTLED``: "your token has exceeded the allowed
    ///   number of calls, or has otherwise been throttled for API abuse."
    /// - ``ERROR_PROCESSING``: "error processing the page. Specific information
    ///   will be returned in the JSON response."
    ApiError(~str, uint),

    /// The JSON was not valid. This is one of those ones that *should* never
    /// happen; you know...
    ///
    /// Actually, I can percieve that it might happen if a document returned
    /// included invalid UTF-8, but this case has not been tested.
    JsonError(json::Error),

    /// An I/O error occurred and the condition was trapped somewhere (by you).
    IoError,
}

impl ToStr for Error {
    fn to_str(&self) -> ~str {
        match *self {
            ApiError(ref msg, code) => format!("API error {}: {}", code, *msg),
            JsonError(ref error) => format!("JSON error: {}", error.to_str()),
            IoError => format!("I/O error (already handled)"),
        }
    }
}

/// Make a simple Diffbot API call.
///
/// For more complex requests, use ``Diffbot`` or ``prepare_request()``.
///
/// Arguments
/// =========
///
/// - ``url`` is the URL that you wish Diffbot to operate upon. If this is a
///   publicly-inaccessible URL, you should use ``post_body()`` on a prepared
///   request instead of ``call()``.
///
/// - ``token`` is the developer's token.
///
/// - ``api`` is the name of the API endpoint, e.g. "article", "product".
///
/// - ``fields`` is the set of fields you want the API call to return; it
///   follows the form specified by the Diffbot API and so should have values
///   like "*", "meta", "querystring", "images(*)".
///
/// - ``version`` is the Diffbot API version number.
pub fn call(url: &Url, token: &str, api: &str, fields: &[&str], version: uint)
           -> Result<json::Object, Error> {
    prepare_request(url, token, api, fields, version).call()
}

/// Prepare, but do not send, a request.
///
/// This allows you to use some of the more advanced features of the API like
/// setting certain headers for Diffbot to use, or uploading a private document
/// for it.
pub fn prepare_request(url: &Url, token: &str, api: &str, fields: &[&str],
                       version: uint)
                      -> Request {
    // First of all we must calculate the GET parameters.
    let mut query = ~[(~"token", token.to_owned()),
                      (~"url", url.to_str())];
    if fields.len() > 0 {
        query.push((~"fields", fields.connect(",")));
    }

    // Now that we've got that, we can figure out the complete URL.
    let url = Url::new(~"http",                          // scheme
                       None,                             // user
                       ~"api.diffbot.com",               // host
                       None,                             // port
                       format!("/v{}/{}", version, api),  // path
                       query,                            // query
                       None);                            // fragment

    // And with that, we can now make the request. Whee!
    Request {
        request: RequestWriter::new(Get, url)
    }
}
