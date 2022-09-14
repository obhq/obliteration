$packages = Get-Content vcpkg-packages.txt

foreach ($package in $packages) {
  .\vcpkg\vcpkg.exe install $package
}
