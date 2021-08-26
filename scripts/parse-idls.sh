#!/usr/bin/env bash

# This script generates the IDL JSONs without buildling the full packages.

rm -fr artifacts/idl/
mkdir -p artifacts/idl/

for PROGRAM in $(find programs/ -maxdepth 3 -name lib.rs); do
    PROGRAM_NAME=$(dirname $PROGRAM | xargs dirname | xargs basename | tr '-' '_')
    echo "Parsing IDL for $PROGRAM_NAME"
    anchor idl parse --file $PROGRAM >artifacts/idl/$PROGRAM_NAME.json || {
        echo "Could not parse IDL"
        exit 1
    }
done
