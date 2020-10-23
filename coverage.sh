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
if ! which genhtml; then
  echo "Error: genthml not found.  Try |brew install lcov| or |apt-get install lcov|"
  exit 1
fi


: "${CI_COMMIT:=local}"
reportName="lcov-${CI_COMMIT:0:9}"

if [[ -z $1 ]]; then
  programs=(
    memo/program
    token/program
    token-lending/program
    token-swap/program
  )
else
  programs=( "$@" )
fi

coverageFlags=(-Zprofile)                # Enable coverage
coverageFlags+=("-Clink-dead-code")      # Dead code should appear red in the report
coverageFlags+=("-Ccodegen-units=1")     # Disable code generation parallelism which is unsupported under -Zprofile (see [rustc issue #51705]).
coverageFlags+=("-Cinline-threshold=0")  # Disable inlining, which complicates control flow.
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

grcov target/cov/tmp > target/cov/lcov-full.info

echo "--- filter-files-from-lcov"

# List of directories to remove from the coverage report
ignored_filepaths="build\.rs"

filter-files-from-lcov() {
  # this function is too noisy for casual bash -x
  set +x
  declare skip=false
  while read -r line; do
    if [[ $line =~ ^SF:/ ]]; then
      skip=true # Skip all absolute paths as these are references into ~/.cargo
    elif [[ $line =~ ^SF:(.*) ]]; then
      declare file="${BASH_REMATCH[1]}"
      if [[ $file =~ $ignored_filepaths ]]; then
        skip=true # Skip paths into ignored locations
      elif [[ -r $file ]]; then
        skip=false
      else
        skip=true # Skip relative paths that don't exist
      fi
    fi
    [[ $skip = true ]] || echo "$line"
  done
}

filter-files-from-lcov < target/cov/lcov-full.info > target/cov/lcov.info

echo "--- html report"

genhtml --output-directory target/cov/$reportName \
  --show-details \
  --highlight \
  --ignore-errors source \
  --prefix "$PWD" \
  --legend \
  target/cov/lcov.info

(
  cd target/cov
  tar zcf report.tar.gz $reportName
)

ls -l target/cov/$reportName/index.html
ln -sfT $reportName target/cov/LATEST

exit $test_status
