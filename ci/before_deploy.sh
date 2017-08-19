# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    cross rustc --bin frogsay --target $TARGET --release -- -C lto

    cp target/$TARGET/release/frogsay $stage/

    cd $stage
    tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *
    cd $src

    if [ $TRAVIS_OS_NAME = linux ]; then
      sudo apt-get -qq update
      sudo apt-get install -y sha256sum
    else
      brew install sha2
    fi

    # Write out the checksum for Homebrew
    HASH=$(sha256sum $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz)
    echo "$HASH" > $CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz-$HASH.sha256

    rm -rf $stage
}

main
