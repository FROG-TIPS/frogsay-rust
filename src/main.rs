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

mod errors;
mod pond;
mod reservoir;
mod speech;
mod tips;

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
    Say(pond::Pond),
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
                Selection::Say(pond::Pond::with_essential_tips())
            } else {
                Selection::Say(pond::Pond::with_cache(true))
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
