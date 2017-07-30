/// Common errors for frogsay.

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
