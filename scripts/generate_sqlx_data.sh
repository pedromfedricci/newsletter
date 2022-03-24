#!/bin/sh
set -e

# NOTE: run on project root only.
if [ ! -f Cargo.toml ]
then
    echo "must run script at project root!"
    exit 1
fi

# NOTE: there is not current way to rename the
# `sqlx-data.json` file name produced by sqlx cli.
# There is also no supported way to generate
# the cached metadata for both lib and test
# targets at the same time.
# So this workflow is needed to generate a
# `sqlx-data.json` file that has all metadata
# for both lib and test targets.

SQLX_DATA_FILE="sqlx-data.json"
SQLX_LIB_FILE="sqlx-data-lib.json"
SQLX_TEST_FILE="sqlx-data-test.json"

SQLX_DATA_FILE_PATH="./${SQLX_DATA_FILE}"
SQLX_LIB_FILE_PATH="./${SQLX_LIB_FILE}"
SQLX_TEST_FILE_PATH="./${SQLX_TEST_FILE}"

# Cache all invocations of `query!` and related
# macros to `sqlx-data.json` for lib target.
cargo sqlx prepare -- --lib
# Rename the file.
mv $SQLX_DATA_FILE_PATH $SQLX_LIB_FILE_PATH
echo "renamed file $SQLX_DATA_FILE to $SQLX_LIB_FILE for lib target"

# Cache all invocations of `query!` and related
# macros to `sqlx-data.json` for test target.
# `api` is our test target.
cargo sqlx prepare -- --test api
# Rename the file.
mv $SQLX_DATA_FILE_PATH $SQLX_TEST_FILE_PATH
echo "renamed file $SQLX_DATA_FILE to $SQLX_TEST_FILE for test target"

# Use `jq` tool to merge the files into
# single `sqlx-data.json` file.
jq -s '.[0] + .[1]' $SQLX_LIB_FILE_PATH $SQLX_TEST_FILE_PATH > $SQLX_DATA_FILE_PATH
echo "merged files $SQLX_LIB_FILE and $SQLX_TEST_FILE to $SQLX_DATA_FILE"

# Cleanup temporaly json files.
rm $SQLX_LIB_FILE_PATH $SQLX_TEST_FILE_PATH
echo "removed temporary files $SQLX_LIB_FILE and $SQLX_TEST_FILE"
