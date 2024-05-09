#!/bin/bash
set -eux

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
pushd $DIR/../althea-info-server
set +e
rm ../scripts/althea-info-server
set -e
cargo build --release
cp target/release/althea-info-server ../scripts
popd

pushd $DIR/../althea-info-dash
yarn run build
rm -rf ../scripts/althea-info-dash/
mkdir ../scripts/althea-info-dash
cp -r build/* ../scripts/althea-info-dash
popd

pushd $DIR
ansible-playbook -i hosts  deploy-info-server.yml
popd

