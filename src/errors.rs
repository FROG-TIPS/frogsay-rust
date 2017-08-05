/// Common errors for frogsay.

error_chain! {
    errors {
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

    links {
        PrivatePath(::private_path::errors::Error, ::private_path::errors::ErrorKind);
    }
}
