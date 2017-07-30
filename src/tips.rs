/// Utilties for loading remote FROG tips.

use futures::{Future, Stream};
use errors::*;
use hyper::{self, Method, Request};
use hyper::header::ContentType;
use hyper_tls;
use tokio_core::reactor::Core;
use serde_json;
use std::io;

/// A tip number, as received from the server. I doubt we'll overflow.
type TipNum = u64;

/// A tip, as received from the server. These are the public API fields.
#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tip {
    pub number: TipNum,
    pub tip: String,
}

/// A croak is normally precisely 50 tips, but serde_json can't handle encoding and decoding
/// fixed-sized arrays, so this contains a vector of tips.
#[derive(Serialize, Deserialize, Debug)]
pub struct Croak {
    pub tips: Vec<Tip>,
}

/// Return a `Result` with a `Croak` of tips.
pub fn croak() -> Result<Croak> {
    let mut core = Core::new()
        .chain_err(|| "Cannot initialize connection pool")?;
    let handle = core.handle();
    let client = hyper::Client::configure()
        .connector(hyper_tls::HttpsConnector::new(1, &handle)
            .chain_err(|| "Cannot initialze TLS")?)
        .build(&handle);

    let uri = "https://frog.tips/api/1/tips/"
        .parse()
        .chain_err(|| "Cannot parse FROG.TIPS URL")?;
    let mut req = Request::new(Method::Get, uri);
    req.headers_mut().set(ContentType::json());
    let get_croak = client.request(req).and_then(|res| {
        res.body().concat2().and_then(move |body| {
            let croak: Croak = serde_json::from_slice(&body)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(croak)
        })
    });
    core.run(get_croak).chain_err(|| "Cannot make request")
}
