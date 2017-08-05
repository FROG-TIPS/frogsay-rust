# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {
    # The application will try to write to $HOME but since Cross uses Docker,
    # it can't write outside the target directory
    if [ $TRAVIS_OS_NAME = linux ]; then
        export HOME=$HOME/target
    fi

    cross build --target $TARGET
    cross build --target $TARGET --release

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    cross test --target $TARGET
    cross test --target $TARGET --release

    cross run --target $TARGET
    cross run --target $TARGET --release
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi
