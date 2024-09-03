#!/usr/bin/env bash

dir=$1
[[ -z $dir ]] && dir='../MoosicBoxUI'

echo "Copying $dir/public"
rm -rf public
mkdir -p public
cp -r "$dir/public/." app-public/. public

srcDirectories=("components" "layouts" "middleware" "pages" "routes" "services" "styles")

for i in "${!srcDirectories[@]}"; do
    directory=${srcDirectories[$i]}

    echo "Copying $dir/src/${directory}"
    rm -rf "src/${directory}"
    mkdir -p "src/${directory}"
    cp -r "$dir/src/${directory}/." "src/app-${directory}/." "src/${directory}"
done

echo "Copying $dir/src/env.d.ts"
cp "$dir/src/env.d.ts" src/env.d.ts
echo "Copying $dir/src/sst-env.d.ts"
cp "$dir/src/sst-env.d.ts" src/sst-env.d.ts
echo "Copying $dir/render-directive"
cp -r "$dir/render-directive" ./
