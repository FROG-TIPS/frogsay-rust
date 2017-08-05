/// Find the right directory for an app to store its data, depending on the OS.

use std::fs;
use std::path::PathBuf;

// Use errors local to this module
use self::errors::*;

pub mod errors {
    error_chain! {
        errors {
            NoHomeDir {
                description("no home directory could be found")
            }
            BadHomeDirEncoding {
                description("the path to the home directory has an unknown encoding")
            }
            PrivatePathNotCreated {
                description("this application's local storage path could not be created")
            }
        }
    }
}

pub struct PrivatePathBuf {
    author: String,
    app_name: String,
    subpath: PathBuf,
}

/// Begin building a path to store private application data. This will take the form of:
/// ```
/// $HOME/<os-specific>/<author>/<app_name/
/// ```
/// where `$HOME` is the current user's home directory and `os-specific` is an OS-specific
/// path. Multiple applications published by the author are nested under the same directory.
pub fn with_author_and_app<S>(author: S, app_name: S) -> PrivatePathBuf
    where S: Into<String> {
    PrivatePathBuf {
        author: author.into(),
        app_name: app_name.into(),
        subpath: PathBuf::new(),
    }
}

impl PrivatePathBuf {
    /// Extends `self` with `path`.
    pub fn push<'a, S>(&'a mut self, path: S) -> &'a mut Self
        where S: Into<String> {
        self.subpath.push(path.into());
        self
    }

    /// If needed, create the full path contained in `self` and then return the path.
    pub fn create(&mut self) -> Result<PathBuf> {
        let mut path = platform::home()?;
        path.push(&self.author);
        path.push(&self.app_name);
        path.push(&self.subpath);

        fs::create_dir_all(path.as_path())
            .chain_err(|| Error::from(ErrorKind::PrivatePathNotCreated))?;

        Ok(path)
    }
}

#[cfg(target_os="macos")]
mod platform {
    use std::env;
    use super::errors::*;

    pub fn home() -> Result<PathBuf> {
        let mut path = PathBuf::new();
        path.push(env::home_dir().ok_or(Error::from(ErrorKind::NoHomeDir))?);
        path.push("Library");
        path.push("Application Support");
        Ok(path)
    }
}

#[cfg(windows)]
mod platform {
    extern crate ole32;
    extern crate shell32;
    extern crate winapi;

    use std::path::PathBuf;
    use std::result;
    use super::errors::*;

    use std::slice;
    use std::ptr;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    // The two functions below are licensed under the following:

    // The MIT License (MIT)
    //
    // Copyright (c) 2016 Andy Barron
    //
    // Permission is hereby granted, free of charge, to any person obtaining a copy
    // of this software and associated documentation files (the "Software"), to deal
    // in the Software without restriction, including without limitation the rights
    // to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    // copies of the Software, and to permit persons to whom the Software is
    // furnished to do so, subject to the following conditions:
    //
    // The above copyright notice and this permission notice shall be included in all
    // copies or substantial portions of the Software.
    //
    // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    // IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    // FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    // AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    // LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    // OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    // SOFTWARE.

    // This value is not currently exported by any of the winapi crates, but
    // its exact value is specified in the MSDN documentation.
    // https://msdn.microsoft.com/en-us/library/dd378457.aspx#FOLDERID_RoamingAppData
    #[allow(non_upper_case_globals)]
    static FOLDERID_RoamingAppData: winapi::GUID = winapi::GUID {
        Data1: 0x3EB685DB,
        Data2: 0x65F9,
        Data3: 0x4CF6,
        Data4: [0xA0, 0x3A, 0xE3, 0xEF, 0x65, 0x72, 0x9F, 0x3D]
    };

    // Retrieves the OsString for AppData using the proper Win32
    // function without relying on environment variables
    fn get_appdata() -> result::Result<OsString, ()> {
        unsafe {
            // A Wide c-style string pointer which will be filled by
            // SHGetKnownFolderPath. We are responsible for freeing
            // this value if the call succeeds
            let mut raw_path: winapi::PWSTR = ptr::null_mut();

            // Get RoamingAppData's path
            let result = shell32::SHGetKnownFolderPath(
                &FOLDERID_RoamingAppData,
                0, // No extra flags are neccesary
                ptr::null_mut(), // user context, null = current user
                &mut raw_path,
            );

            // SHGetKnownFolderPath returns an HRESULT, which represents
            // failure states by being negative. This should not fail, but
            // we should be prepared should it fail some day.
            if result < 0 {
                return Err(());
            }

            // Since SHGetKnownFolderPath succeeded, we must ensure that we
            // free the memory even if allocating an OsString fails later on.
            // To do this, we will use a nested struct with a Drop implementation
            let _cleanup = {
                struct FreeStr(winapi::PWSTR);
                impl Drop for FreeStr {
                    fn drop(&mut self) {
                        unsafe { ole32::CoTaskMemFree(self.0 as *mut _) };
                    }
                }
                FreeStr(raw_path)
            };

            // libstd does not contain a wide-char strlen as far as I know,
            // so we'll have to make do calculating it ourselves.
            let mut strlen = 0;
            for i in 0.. {
                if *raw_path.offset(i) == 0 {
                    // isize -> usize is always safe here because we know
                    // that an isize can hold the positive length, as each
                    // char is 2 bytes long, and so could only be half of
                    // the memory space even theoretically.
                    strlen = i as usize;
                    break;
                }
            }

            // Now that we know the length of the string, we can
            // convert it to a &[u16]
            let wpath = slice::from_raw_parts(raw_path, strlen);
            // Window's OsStringExt has the function from_wide for
            // converting a &[u16] into an OsString.
            let path = OsStringExt::from_wide(wpath);

            // raw_path will be automatically freed by _cleanup, regardless of
            // whether any of the previous functions panic.

            Ok(path)
        }
    }

    pub fn home() -> Result<PathBuf> {
        let mut path = PathBuf::new();
        path.push(
            get_appdata()
                .map_err(|_| Error::from(ErrorKind::NoHomeDir))?
                .into_string()
                .map_err(|_| Error::from(ErrorKind::BadHomeDirEncoding))?);
        Ok(path)
    }
}

#[cfg(not(any(windows, unix, target_os="macos",)))]
mod platform {
    use std::env;
    use super::errors::*;

    pub fn home() -> Result<PathBuf> {
        // Try our best on all other OSes.
        // Since nothing is standard on Linux, putting files under .local seems reasonable
        let mut path = PathBuf::new();
        path.push(env::home_dir().ok_or(Error::from(ErrorKind::NoHomeDir))?);
        path.push(".local");
        Ok(path)
    }
}
