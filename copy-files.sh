#!/usr/bin/env bash

dir=$1
[[ -z $dir ]] && dir='../MoosicBoxUI'

echo "Copying $dir/public"
rm -rf public
mkdir -p public
cp -rT "$dir/public" public
cp -rT app-public/. public
echo "Copying $dir/src/components"
rm -rf src/components
mkdir -p src/components
cp -rT "$dir/src/components" src/components
cp -rT src/app-components/. src/components
echo "Copying $dir/src/layouts"
rm -rf src/layouts
mkdir -p src/layouts
cp -rT "$dir/src/layouts" src/layouts
cp -rT src/app-layouts/. src/layouts
echo "Copying $dir/src/middleware"
rm -rf src/middleware
mkdir -p src/middleware
cp -rT "$dir/src/middleware" src/middleware
cp -rT src/app-middleware/. src/middleware
echo "Copying $dir/src/pages"
rm -rf src/pages
mkdir -p src/pages
cp -rT "$dir/src/pages" src/pages
cp -rT src/app-pages/. src/pages
echo "Copying $dir/src/routes"
rm -rf src/routes
mkdir -p src/routes
cp -rT "$dir/src/routes" src/routes
cp -rT src/app-routes/. src/routes
echo "Copying $dir/src/services"
rm -rf src/services
mkdir -p src/services
cp -rT "$dir/src/services" src/services
cp -rT src/app-services/. src/services
echo "Copying $dir/src/styles"
rm -rf src/styles
mkdir -p src/styles
cp -rT "$dir/src/styles" src/styles
cp -rT src/app-styles/. src/styles
echo "Copying $dir/src/env.d.ts"
cp "$dir/src/env.d.ts" src/env.d.ts
echo "Copying $dir/src/sst-env.d.ts"
cp "$dir/src/sst-env.d.ts" src/sst-env.d.ts
echo "Copying $dir/render-directive"
cp -r "$dir/render-directive" ./
