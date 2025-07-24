#!/bin/bash

set -x

git pull

cargo build --release --bin collect

git clone git@github.com:bevyengine/bevy.git
cd bevy

git clone -b results git@github.com:bevyengine/twitcher.git results
find results -mindepth 3 -maxdepth 3 '!' -exec test -e "{}/wasm32-unknown-unknown-size.stats" ';' -print | grep -v git 2> /dev/null > /dev/null
has_work=$?
[ $has_work -eq 1 ] && exit 1

gitref=`find results -mindepth 3 -maxdepth 3 '!' -exec test -e "{}/wasm32-unknown-unknown-size.stats" ';' -print | grep -v git | head -n 1 | cut -d '/' -f 4`

git reset --hard $gitref
../target/release/collect --merge-results wasm-binary-size

cd results
git add .
git commit -m "Add wasm-binary-size for $gitref"
git push
cd ..
rm -rf results

cd ..
rm -rf bevy
