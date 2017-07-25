//! Frogsay is a program that shows an ASCII-art frog spouting pithy wisdom from frog.tips.
//!
//! This special deluxe version of frogsay contains "essential" tips, which are hardcoded into
//! the program, alleviating the need for network access to provide a tip.

// error-chain requires deep recursion
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate serde_derive;

extern crate itertools;
extern crate getopts;
extern crate app_dirs;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate textwrap;
extern crate tokio_core;

/// Common errors for frogsay.
mod errors {
    error_chain! {
        errors {
            CachePathNotCreated {
                description("the path to the cache could not be created")
            }
            CacheNotCreated {
                description("the cache could not be created")
            }
            CacheNotUpdated {
                description("the cache could not be updated")
            }
            NoTips {
                description("no tips were available")
            }
            NoEssentialTips {
                description("essential tips were not available")
            }
        }
    }
}

/// Utilities for the reservoir data structure.
mod reservoir {
    use errors::*;

    /// A data structure that can be drained of its items until empty, then automatically refilled
    /// with a provided `fill` function.
    pub trait Reservoir<V> {
        /// Returns an `Ok` containing the next item in the reservoir or `Err` if it could
        /// not be provided.
        ///
        /// # Arguments
        /// * `fill_fn` - A function used to refill the reservoir with items. This may be called
        ///               zero or more times by the implemenation.`
        fn next_or_fill<F>(self, fill_fn: F) -> Result<V>
        where
            F: Fn() -> Result<Vec<V>>;
    }
}

/// Utilties for loading remote FROG tips.
mod tips {
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
}

/// Utilities for formatting the spouting frog.
mod speech {
    use itertools::Itertools;
    use textwrap::Wrapper;

    /// Return a string with a frog spouting the given `text` in a speech bubble. The speech
    /// text will be wrapped to the current terminal width. Distinct paragraphs are supported and
    /// can be specified by inserting newlines in `text`. Whitespace immediately after or before
    /// each newline will be trimmed.
    ///
    /// # Arguments
    /// - `text` - Newline-separated text to display.
    pub fn say<S>(text: S) -> String
    where
        S: Into<String>,
    {
        let indent = "        ";
        let wrapper = Wrapper::with_termwidth()
            .subsequent_indent(indent)
            .initial_indent(indent)
            .squeeze_whitespace(false)
            .break_words(true);

        let wrapped = text.into()
            .lines()
            .map(|p| wrapper.fill(&format!("{}", p)))
            .join("\n");

        format!(
            r#"
{text}
{indent}/
  @..@
 (----)
( >__< )
^^ ~~ ^^"#,
            indent = indent,
            text = wrapped
        )
    }
}

/// Everything related to the command-line application.
mod frogsay {
    use app_dirs;
    use errors::*;
    use rand::{self, Rng};
    use reservoir::{self, Reservoir};
    use serde_json;
    use speech;
    use std::fs;
    use std::io::{Seek, SeekFrom};
    use std::result;
    use tips;

    /// The cache file is versioned for future upgrading and recovery.
    const CACHE_VERSION: &'static str = "1";
    static ESSENTIAL_JSON: &'static str = include_str!("../txt/essential.json");

    pub struct Pond {
        mode: ReservoirMode,
    }

    enum ReservoirMode {
        Cache,
        EssentialCache,
        NoCache,
    }

    impl Pond {
        pub fn with_cache(cache_enabled: bool) -> Pond {
            Pond {
                mode: if cache_enabled {
                    ReservoirMode::Cache
                } else {
                    ReservoirMode::NoCache
                },
            }
        }

        pub fn with_essential_tips() -> Pond {
            Pond {
                mode: ReservoirMode::EssentialCache,
            }
        }

        pub fn say(self) -> Result<()> {
            let tip = self.next_or_fill(|| Ok(tips::croak()?.tips))?;
            Ok(println!("{}", speech::say(tip.tip)))
        }
    }

    impl reservoir::Reservoir<tips::Tip> for Pond {
        fn next_or_fill<F>(self, fill_fn: F) -> Result<tips::Tip>
        where
            F: Fn() -> Result<Vec<tips::Tip>>,
        {
            match self.mode {
                ReservoirMode::Cache => {
                    let mut cache = {
                        let file = app_dirs::app_dir(
                            app_dirs::AppDataType::UserCache,
                            &::APP_INFO,
                            "cache",
                        ).chain_err(|| ErrorKind::CachePathNotCreated)?
                            .join(format!("cache-{}.json", CACHE_VERSION));

                        fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(file)
                            .chain_err(|| ErrorKind::CacheNotCreated)?
                    };

                    let json: result::Result<Vec<tips::Tip>, serde_json::Error> =
                        serde_json::from_reader(&mut cache);
                    let mut tips = match json {
                        Ok(contents) => {
                            match contents.len() {
                                0 => fill_fn()?,
                                _ => contents,
                            }
                        }
                        // Couldn't read the cache. Eh.
                        _ => fill_fn()?,
                    };

                    // By this point we should have at least one tip. If no tips are available, it means the server is returning zero tips
                    // or the cache file has been cleared by another process (FROG IS NOT THREADSAFE).
                    let tip = tips.pop().ok_or(Error::from(ErrorKind::NoTips))?;

                    cache
                        .set_len(0)
                        .and_then(|_| cache.seek(SeekFrom::Start(0)))
                        .chain_err(|| ErrorKind::CacheNotUpdated)?;

                    serde_json::to_writer(cache, &tips)
                        .chain_err(|| ErrorKind::CacheNotUpdated)?;

                    Ok(tip)
                }
                ReservoirMode::NoCache => fill_fn()?.pop().ok_or(Error::from(ErrorKind::NoTips)),
                ReservoirMode::EssentialCache => {
                    let tips: Vec<tips::Tip> = serde_json::from_str(ESSENTIAL_JSON)
                        .chain_err(|| ErrorKind::NoEssentialTips)?;
                    rand::thread_rng()
                        .choose(&tips)
                        .cloned()
                        .ok_or(Error::from(ErrorKind::NoTips))
                }
            }
        }
    }
}

use std::io::{self, Write};
use getopts::Options;
use std::env;

const APP_INFO: app_dirs::AppInfo = app_dirs::AppInfo {
    name: env!("CARGO_PKG_NAME"),
    author: "FROG SYSTEMS",        // app_dirs will nest files under the folders {author}/{name}
};

enum Selection {
    Usage,
    Version,
    Say(frogsay::Pond),
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("e", "essential", "SHOW ONLY ESSENTIAL TIPS.");
    opts.optflag("h", "help", "THIS HELP.");
    opts.optflag("v", "version", "SHOW THE PROGRAM'S VERSION.");

    let selection = match opts.parse(&args[1..]) {
        Ok(matches) => {
            if matches.opt_present("h") {
                Selection::Usage
            } else if matches.opt_present("v") {
                Selection::Version
            } else if matches.opt_present("e") {
                Selection::Say(frogsay::Pond::with_essential_tips())
            } else {
                Selection::Say(frogsay::Pond::with_cache(true))
            }
        }
        Err(_) => Selection::Usage,
    };

    ::std::process::exit(match selection {
        Selection::Usage => {
            let usage = opts.usage(
        format!("\
FROGSAY IS A PROGRAM THAT GENERATES AN ASCII PICTURE OF A FROG SPOUTING A FROG TIP.

USAGE: {program} [OPTIONS]

FROG TIPS ARE FETCHED FROM HTTPS://FROG.TIPS'S API ENDPOINT. IF THE \"ESSENTIAL\" FLAG IS GIVEN, ONLY THE ESSENTIAL FROG TIPS, HARDCODED INTO THIS PROGRAM, WILL BE SHOWN AND NO NETWORK REQUESTS WILL BE MADE. THIS MAKES FROGSAY SUITABLE FOR INSTALLATION IN SECURE FACILITIES SUCH AS DATA CENTERS AND SPACECRAFT.", program=program).as_ref());
            println!("{}", speech::say(usage));
            1
        }
        Selection::Version => {
            println!("frogsay {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Selection::Say(pond) => {
            match pond.say() {
                Err(why) => {
                    // println! will panic if it fails to write to stdout, so do the same here
                    write!(io::stderr(), "{}", speech::say(format!("error: {}", why).to_uppercase().as_ref())).unwrap();
                    2
                },
                _ => 0,
            }
        }
    })
}
