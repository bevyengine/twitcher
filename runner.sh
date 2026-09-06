#!/bin/bash

set -x

host=$(hostname)
config="${MACHINE_CONFIG:-../$host.toml}"

if [ ! -f "$config" ]; then
    echo "no machine config at $config" >&2
    exit 1
fi

suites=$(sed -n 's/^suites *= *\[\(.*\)\].*/\1/p' "$config" | tr -d '" ' | tr ',' ' ')

if [ -z "$suites" ]; then
    echo "could not read suites from $config" >&2
    exit 1
fi

if [ -x /usr/local/sbin/pin.sh ]; then
    if ! /usr/local/sbin/pin.sh verify; then
        echo "pinned profile mismatch - re-applying once" >&2
        sudo /usr/local/sbin/pin.sh apply
        if ! /usr/local/sbin/pin.sh verify; then
            echo "pinned profile still wrong, skipping this run" >&2
            exit 1
        fi
    fi
else
    echo "warning: /usr/local/sbin/pin.sh not installed, running unpinned" >&2
fi

git pull

cargo build --release --bin collect

git clone -b queue git@github.com:bevyengine/twitcher.git queue
gitref=$(
    for suite in $suites; do
        [ -d "queue/$suite" ] || continue
        for entry in "queue/$suite"/*; do
            [ -f "$entry" ] && basename "$entry"
        done
    done | sort | uniq -c | sort -k1,1rn -k2,2 | head -n 1 | awk '{print $2}'
)

if [ -z "$gitref" ]; then
    rm -rf queue
    echo "no queued work for suites: $suites"
    exit 1
fi

run_suites=""
for suite in $suites; do
    if [ -f "queue/$suite/$gitref" ]; then
        run_suites="${run_suites:+$run_suites }$suite"
    fi
done
run_suites_csv=$(echo "$run_suites" | tr ' ' ',')

git clone git@github.com:bevyengine/bevy.git
cd bevy
git reset --hard "$gitref"
../target/release/collect --suites "$run_suites_csv" all
cd ..

git clone -b results git@github.com:bevyengine/twitcher.git results
cp -r bevy/results/* results
cd results
git add .
git commit -m "Add $run_suites_csv results for $gitref ($host)"
git push
cd ..

cd queue
git pull
for suite in $run_suites; do
    rm -f "$suite/$gitref"
done
git add .
git commit -m "Done $run_suites_csv for $gitref ($host)"
git push
cd ..

rm -rf queue
rm -rf results
rm -rf bevy
