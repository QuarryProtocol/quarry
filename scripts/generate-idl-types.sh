#!/usr/bin/env bash

shopt -s extglob

cd $(dirname $0)/..

generate_declaration_file() {
    PROGRAM_TS=$1
    OUT_DIR=$2

    prog="$(basename $PROGRAM_TS .ts)"
    OUT_PATH="$OUT_DIR/$prog.ts"
    if [ ! $(which gsed) ]; then
        PREFIX=$(echo $prog | sed -E 's/(^|_)([a-z])/\U\2/g')
    else
        PREFIX=$(echo $prog | gsed -E 's/(^|_)([a-z])/\U\2/g')
    fi
    typename="${PREFIX}IDL"
    rawName="${PREFIX}JSON"

    cat $PROGRAM_TS >"$OUT_DIR/$prog.raw.ts"

    # types
    echo "import type { $PREFIX as $typename } from './$prog.raw';" >>$OUT_PATH
    echo "import { IDL as $rawName } from './$prog.raw';" >>$OUT_PATH

    echo "export { $typename, $rawName };" >>$OUT_PATH

    # error type
    echo "import { generateErrorMap } from '@saberhq/anchor-contrib';" >>$OUT_PATH
    echo "export const ${PREFIX}Errors = generateErrorMap($rawName);" >>$OUT_PATH
}

generate_sdk_idls() {
    SDK_DIR=${1:-"./packages/sdk/src/idls"}
    IDL_JSONS=$2

    echo "Generating IDLs for the following programs:"
    echo $IDL_JSONS
    echo ""

    rm -rf $SDK_DIR
    mkdir -p $SDK_DIR
    if [ $(ls -l artifacts/idl/ | wc -l) -ne 0 ]; then
        for f in $IDL_JSONS; do
            generate_declaration_file $f $SDK_DIR
        done
        if [[ $RUN_ESLINT != "none" ]]; then
            yarn eslint --fix $SDK_DIR
        fi
    else
        echo "Warning: no IDLs found. Make sure you ran ./scripts/idl.sh first."
    fi
}

generate_sdk_idls ./src/idls 'artifacts/idl/*.ts'
