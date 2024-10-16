#!/bin/sh

DIRECTORY=$1
EXPECTED_NUMBER_ERRORS=$2
TEST_DIR=$(mktemp -d)

cargo build

echo "testing on ${DIRECTORY}"
echo "test dir on ${TEST_DIR}"

osv-scanner  --skip-git -r --experimental-only-packages --format=cyclonedx-1-5 --paths-relative-to-scan-dir --output "${TEST_DIR}/osv-scanner.json" "${DIRECTORY}"
./target/release/sbom-generator --directory "${DIRECTORY}" --output "${TEST_DIR}/sbom-generator.json"

misc/compare-sbom.py "${TEST_DIR}/osv-scanner.json" "${TEST_DIR}/sbom-generator.json"
ACTUAL_NUMBER_ERRORS=$?
if [ "${ACTUAL_NUMBER_ERRORS}" != "${EXPECTED_NUMBER_ERRORS}" ]; then
  echo "Number of errors mismatch. Expected ${EXPECTED_NUMBER_ERRORS}, got ${ACTUAL_NUMBER_ERRORS}"
  exit 1
fi
exit 0