/// Everything related to the command-line application.
use errors::*;
use rand::{self, Rng};
use reservoir::{self, Reservoir};
use serde_json;
use speech;
use std::fs;
use std::io::{Seek, SeekFrom};
use std::result;
use tips;
use private_path;

/// The cache file is versioned for future upgrading and recovery.
const CACHE_VERSION: &'static str = "1";

const PROGRAM_AUTHOR: &'static str = "FROG SYSTEMS";
const PROGRAM_NAME: &'static str = env!("CARGO_PKG_NAME");

static ESSENTIAL_JSON: &'static str = include_str!("../txt/essential.json");

pub struct Pond {
    mode: ReservoirMode,
}

enum ReservoirMode {
    Cache,
    EssentialCache,
}

impl Pond {
    pub fn with_cache() -> Pond {
        Pond {
            mode: ReservoirMode::Cache,
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
                    let path = private_path::with_author_and_app(PROGRAM_NAME, PROGRAM_AUTHOR)
                        .push("cache")
                        .create()?
                        .join(format!("cache-{}.json", CACHE_VERSION));

                    fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(path)
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
            },
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
