#!/usr/bin/env bash

dir=$1
[[ -z $dir ]] && dir='../MoosicBoxUI'

echo "Copying $dir/src/components"
rm -rf src/components && cp -r "$dir/src/components" src/components
echo "Copying $dir/src/pages"
rm -rf src/pages && cp -r "$dir/src/pages" src/pages
echo "Copying $dir/src/routes"
rm -rf src/routes && cp -r "$dir/src/routes" src/routes
echo "Copying $dir/src/services"
rm -rf src/services && cp -r "$dir/src/services" src/services
echo "Copying $dir/public"
rm -rf public && cp -r "$dir/public" public
