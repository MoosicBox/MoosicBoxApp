#!/usr/bin/env bash

dir=$1
[[ -z $dir ]] && dir='../MoosicBoxUI'

echo "Copying $dir/public"
rm -rf public
mkdir -p public
cp -r "$dir/public/." app-public/. public
echo "Copying $dir/src/components"
rm -rf src/components
mkdir -p src/components
cp -r "$dir/src/components/." src/app-components/. src/components
echo "Copying $dir/src/layouts"
rm -rf src/layouts
mkdir -p src/layouts
cp -r "$dir/src/layouts/." src/app-layouts/. src/layouts
echo "Copying $dir/src/middleware"
rm -rf src/middleware
mkdir -p src/middleware
cp -r "$dir/src/middleware/." src/app-middleware/. src/middleware
echo "Copying $dir/src/pages"
rm -rf src/pages
mkdir -p src/pages
cp -r "$dir/src/pages/." src/app-pages/. src/pages
echo "Copying $dir/src/routes"
rm -rf src/routes
mkdir -p src/routes
cp -r "$dir/src/routes/." src/app-routes/. src/routes
echo "Copying $dir/src/services"
rm -rf src/services
mkdir -p src/services
cp -r "$dir/src/services/." src/app-services/. src/services
echo "Copying $dir/src/styles"
rm -rf src/styles
mkdir -p src/styles
cp -r "$dir/src/styles/." src/app-styles/. src/styles
echo "Copying $dir/src/env.d.ts"
cp "$dir/src/env.d.ts" src/env.d.ts
echo "Copying $dir/src/sst-env.d.ts"
cp "$dir/src/sst-env.d.ts" src/sst-env.d.ts
echo "Copying $dir/render-directive"
cp -r "$dir/render-directive" ./
