use std::borrow::Cow;
use std::collections::HashMap;
use request::Request;
use urlencoded;
use hyper::uri::RequestUri;
use hyper::uri::RequestUri::{Star, AbsoluteUri, AbsolutePath, Authority};
use url::UrlParser;
use plugin::{Plugin, Pluggable};
use typemap::Key;

type QueryStore = HashMap<String, Vec<String>>;

#[derive(Debug, PartialEq, Eq)]
pub struct Query(QueryStore);

impl Query {
    pub fn get(&self, key: &str) -> Option<&[String]> {
        self.0.get(key).map(|v| &**v)
    }

    pub fn get_or(&self, key: &str, default: &str) -> Cow<[String]> {
        match self.0.get(key) {
            Some(result) => Cow::Borrowed(result),
            None => Cow::Owned(vec![default.to_string()])
        }
    }
}

// Plugin boilerplate
struct QueryStringParser;
impl Key for QueryStringParser { type Value = Query; }

impl<'a, 'b, 'k> Plugin<Request<'a, 'b, 'k>> for QueryStringParser {
    type Error = ();

    fn eval(req: &mut Request) -> Result<Query, ()> {
        Ok(parse(&req.origin.uri))
    }
}

pub trait QueryString {
    fn query(&mut self) -> &Query;
}

impl<'a, 'b, 'k> QueryString for Request<'a, 'b, 'k> {
    fn query(&mut self) -> &Query {
        self.get_ref::<QueryStringParser>()
            .ok()
            .expect("Bug: QueryStringParser returned None")
    }
}

fn parse(origin: &RequestUri) -> Query {
    let f = |query: Option<&String>| query.map(|q| urlencoded::parse(&*q));

    let result = match *origin {
        AbsoluteUri(ref url) => f(url.query.as_ref()),
        AbsolutePath(ref s) => UrlParser::new().parse_path(&*s)
                                                // FIXME: If this fails to parse,
                                                // then it really shouldn't have
                                                // reached here.
                                               .ok()
                                               .and_then(|(_, query, _)| f(query.as_ref())),
        Star | Authority(..) => None
    };

    Query(result.unwrap_or_else(|| HashMap::new()))
}

#[test]
fn splits_and_parses_an_url() {
    use url::Url;
    let t = |url| {
        let store = parse(&url);
        assert_eq!(store.get("foo"), Some(&["bar".to_string()][..]));
        assert_eq!(store.get_or("foo", "other"), &["bar".to_string()][..]);
        assert_eq!(store.get_or("bar", "other"), &["other".to_string()][..]);
        assert_eq!(store.get("message"),
                        Some(&["hello".to_string(), "world".to_string()][..]));
    };

    let raw = "http://www.foo.bar/query/test?foo=bar&message=hello&message=world";
    t(AbsoluteUri(Url::parse(raw).unwrap()));

    t(AbsolutePath("/query/test?foo=bar&message=hello&message=world".to_string()));

    assert_eq!(parse(&Star), Query(HashMap::new()));

    let store = parse(&Authority("host.com".to_string()));
    assert_eq!(store, Query(HashMap::new()));
}
