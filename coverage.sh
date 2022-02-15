#!/usr/bin/env bash
#
# Runs all program tests and builds a code coverage report
#

set -e
cd "$(dirname "$0")"

if ! which grcov; then
  echo "Error: grcov not found.  Try |cargo install grcov|"
  exit 1
fi

if [[ ! "$(grcov --version)" =~ 0.[678].[0124] ]]; then
  echo Error: Required grcov version not installed
  exit 1
fi

: "${CI_COMMIT:=local}"
reportName="lcov-${CI_COMMIT:0:9}"

if [[ -z $1 ]]; then
  programs=(
    libraries/math
    memo/program
    token/program
    token-lending/program
    token-swap/program
  )
else
  programs=("$@")
fi

coverageFlags=(-Zprofile)                # Enable coverage
coverageFlags+=("-Clink-dead-code")      # Dead code should appear red in the report
coverageFlags+=("-Ccodegen-units=1")     # Disable code generation parallelism which is unsupported under -Zprofile (see [rustc issue #51705]).
coverageFlags+=("-Cinline-threshold=0")  # Disable inlining, which complicates control flow.
coverageFlags+=("-Copt-level=0")         #
coverageFlags+=("-Coverflow-checks=off") # Disable overflow checks, which create unnecessary branches.

export RUSTFLAGS="${coverageFlags[*]} $RUSTFLAGS"
export CARGO_INCREMENTAL=0
export RUST_BACKTRACE=1
export RUST_MIN_STACK=8388608

echo "--- remove old coverage results"
if [[ -d target/cov ]]; then
  find target/cov -type f -name '*.gcda' -delete
fi
rm -rf target/cov/$reportName
mkdir -p target/cov

# Mark the base time for a clean room dir
touch target/cov/before-test

for program in ${programs[@]}; do
  here=$PWD
  (
    set -ex
    cd $program
    cargo +nightly test --target-dir $here/target/cov
  )
done

touch target/cov/after-test

echo "--- grcov"

# Create a clean room dir only with updated gcda/gcno files for this run,
# because our cached target dir is full of other builds' coverage files
rm -rf target/cov/tmp
mkdir -p target/cov/tmp

# Can't use a simpler construct under the condition of SC2044 and bash 3
# (macOS's default). See: https://github.com/koalaman/shellcheck/wiki/SC2044
find target/cov -type f -name '*.gcda' -newer target/cov/before-test ! -newer target/cov/after-test -print0 |
  (while IFS= read -r -d '' gcda_file; do
    gcno_file="${gcda_file%.gcda}.gcno"
    ln -sf "../../../$gcda_file" "target/cov/tmp/$(basename "$gcda_file")"
    ln -sf "../../../$gcno_file" "target/cov/tmp/$(basename "$gcno_file")"
  done)

(
  set -x
  grcov target/cov/tmp --llvm -t html -o target/cov/$reportName
  grcov target/cov/tmp --llvm -t lcov -o target/cov/lcov.info

  cd target/cov
  tar zcf report.tar.gz $reportName
)

ls -l target/cov/$reportName/index.html
ln -sfT $reportName target/cov/LATEST

exit $test_status
