#!/bin/sh

for ts in simple hierarchy; do
  cargo run -- --directory resources/maven/$ts/ --output /tmp/sbom-$ts-test.json
  python misc/compare-sbom.py /tmp/sbom-$ts-test.json resources/maven/$ts/results/osv.json

  if [ $? -ne 0 ]; then
    echo "TEST $ts FAILED"
  else
    echo "TEST $ts SUCCEEDED"
  fi
done
