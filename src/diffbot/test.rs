extern mod diffbot = "diffbot#1.0";
extern mod extra = "extra#0.10-pre";

use extra::json;
use extra::treemap::TreeMap;

fn invalid_token() -> ~str {
    ~"0"
}

fn valid_token() -> ~str {
    ~"6932269b31d051457940f3da4ee23b79"
}

#[test]
pub fn invalid_key() {
    assert_eq!(diffbot::call(&from_str("http://example.com/").unwrap(),
                             invalid_token(), "frontpage", [], 2),
               Err(diffbot::ApiError(~"Not authorized API token.", diffbot::UNAUTHORIZED_TOKEN)));
}

/// Test the frontpage API on a real, live website.
#[test]
pub fn test_frontpage() {
    // We do quite a bit of error-checking here, more than would usually be done.
    match diffbot::call(&from_str("http://example.com/").unwrap(),
                        valid_token(), "frontpage", [], 2) {
        Err(diffbot::ApiError(msg, diffbot::UNAUTHORIZED_TOKEN)) =>
            fail!("Uh oh, that token isn't authorized. (Full message: {})", msg),
        Err(diffbot::ApiError(msg, diffbot::REQUESTED_PAGE_NOT_FOUND)) =>
            fail!("I am sad, for that page wasn't found. (Full message: {})", msg),
        Err(diffbot::ApiError(msg, diffbot::TOKEN_EXCEEDED_OR_THROTTLED)) =>
            fail!("Hey, hey... slow down, pal! (Full message: {})", msg),
        Err(diffbot::ApiError(msg, diffbot::ERROR_PROCESSING)) =>
            fail!("Oh noes! You (maybe) broke the Diffbot! (Full message: {})", msg),
        Err(diffbot::ApiError(msg, code)) =>
            fail!("Oh noes! Something went wrong, and I don't know what. \
                   (Unknown error; code: {}, full message: {})", code, msg),
        Err(diffbot::JsonError(error)) =>
            fail!("Uh oh, Diffbot returned invalid JSON! (Did you do something wrong?) \
                   Full message: {}", error.to_str()),
        Err(diffbot::IoError) => unreachable!(),  // ... because we didn't trap the condition.
        Ok(object) => {
            let mut expected = TreeMap::new();
            expected.insert(~"sections", json::List(~[{
                let mut section = TreeMap::new();
                section.insert(~"items", json::List(~[{
                    let mut item = TreeMap::new();
                    item.insert(~"title", json::String(~"Example Domain"));
                    item.insert(~"url", json::String(~"http://www.iana.org/domains/example"));
                    json::Object(~item)
                }]));
                section.insert(~"primary", json::Boolean(true));
                json::Object(~section)
            }]));
            expected.insert(~"title", json::String(~"Example Domain"));
            expected.insert(~"url", json::String(~"http://example.com/"));
            assert_eq!(object, expected);
        },
    }
}

/// Test prepared requests, posting a body, and limiting the fields to return.
#[test]
fn test_post_body_and_fields() {
    let request = diffbot::prepare_request(&from_str("http://example.com/").unwrap(),
                                           valid_token(),
                                           "article",
                                           ["title", "images(url)"],
                                           2);
    let body = bytes!("<title>Contents of title tag</title>
                       <h1>Contents of heading tag</h1>
                       <p>Contents of big body
                       <img src=//example.org/example.jpg>
                       <img src=example.png>
                       <p>More page contents");
    let mut response = request.post_body(body).unwrap();

    assert_eq!(response.pop(&~"type"),
               Some(json::String(~"article")));
    assert_eq!(response.pop(&~"url"),
               Some(json::String(~"http://example.com/")));
    assert_eq!(response.pop(&~"title"),
               Some(json::String(~"Contents of heading tag")));

    // Get the URL of each image
    let image_urls = match response.pop(&~"images").unwrap() {
        json::List(images) => images.move_iter().map(|image| match image {
            json::Object(~mut o) => {
                assert_eq!(o.len(), 1);
                match o.pop(&~"url").unwrap() {
                    json::String(ref s) => s.to_owned(),
                    _ => unreachable!(),
                }
            },
            _ => unreachable!(),
        }),
        _ => unreachable!(),
    }.collect::<~[~str]>();

    assert_eq!(image_urls, ~[~"http://example.org/example.jpg",
                             ~"http://example.com/example.png"]);

    // And those were the only fields reurned.
    assert_eq!(response.len(), 0);
}

static TIMEOUT_MESSAGE: &'static str = "Request timed out. The \"timeout\" \
    query string option can be used to modify the timeout (in milliseconds). \
    For example, \"...?timeout=5000&url=...\"";

/// Test the Diffbot struct and timeouts on a prepared request.
#[test]
fn test_diffbot_struct_and_timeouts() {
    let diffbot = diffbot::Diffbot::new(valid_token(), 2);
    let mut request = diffbot.prepare_request(&from_str("http://example.com/").unwrap(),
                                              "article", []);
    // I think we're fairly safe that example.com won't respond in 1ms.
    request.timeout(1);
    assert_eq!(request.call(),
               Err(diffbot::ApiError(TIMEOUT_MESSAGE.to_owned(), 500)));
}
