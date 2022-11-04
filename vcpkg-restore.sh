#!/bin/sh -e
while read -r pkg
do
  ./vcpkg/vcpkg install "$pkg"
done < vcpkg-packages.txt
