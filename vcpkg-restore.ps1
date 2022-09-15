$packages = Get-Content vcpkg-packages.txt

foreach ($package in $packages) {
  .\vcpkg\vcpkg.exe --triplet=x64-windows --vcpkg-root=vcpkg install $package
}
