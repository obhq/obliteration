#!/usr/bin/env python3
from argparse import ArgumentParser
import json
import os
import platform
import shutil
from subprocess import PIPE, Popen, run
import sys
from urllib.parse import urlparse

def cargo(package, toolchain=None, target=None, release=False, args=None):
    # Get package ID.
    cmd = ['cargo']

    if toolchain is not None:
        cmd.append(f'+{toolchain}')

    id = run(cmd + ['pkgid', '-p', package], stdout=PIPE, check=True).stdout.decode('utf-8').strip()

    # Parse package ID.
    url = urlparse(id)
    path = url.netloc + url.path

    # Setup command and its argument.
    cmd.extend(['build', '-p', package])

    if target is not None:
        cmd.extend(['--target', target])

    if release:
        cmd.append('-r')

    if args is not None:
        cmd.extend(args)

    cmd.extend(['--message-format', 'json-render-diagnostics'])

    # Run.
    with Popen(cmd, stdout=PIPE, cwd=path) as proc:
        for line in proc.stdout:
            line = json.loads(line)
            reason = line['reason']
            if reason == 'build-finished':
                if line['success']:
                    break
                else:
                    sys.exit(1)
            elif reason == 'compiler-artifact':
                if line['package_id'] == id:
                    artifact = line

    return artifact

def export_darwin(root, kern, gui):
    # Create directories.
    bundle = os.path.join(root, 'Obliteration.app')
    contents = os.path.join(bundle, 'Contents')
    macos = os.path.join(contents, 'MacOS')
    resources = os.path.join(contents, 'Resources')

    os.mkdir(bundle)
    os.mkdir(contents)
    os.mkdir(macos)
    os.mkdir(resources)

    # Export files
    shutil.copy(kern['executable'], resources)
    shutil.copy(gui['executable'], macos)
    shutil.copyfile('bundle.icns', os.path.join(resources, 'obliteration.icns'))
    shutil.copy('Info.plist', contents)

    # Sign bundle.
    run(['codesign', '-s', '-', '--entitlements', 'entitlements.plist', bundle], check=True)

def export_linux(root, kern, gui):
    # Create directories.
    bin = os.path.join(root, 'bin')
    share = os.path.join(root, 'share')

    os.mkdir(bin)
    os.mkdir(share)

    # Export files.
    shutil.copy(kern['executable'], share)
    shutil.copy(gui['executable'], bin)

def export_windows(root, kern, gui):
    # Create share directory.
    share = os.path.join(root, 'share')

    os.mkdir(share)

    # Export files.
    shutil.copy(kern['executable'], share)
    shutil.copy(gui['executable'], root)

def main():
    # Setup argument parser.
    p = ArgumentParser(
        description='Script to build Obliteration and create distribution file')

    p.add_argument('-r', '--release', action='store_true', help='enable optimization')
    p.add_argument(
        '--root',
        metavar='PATH',
        help='directory to store build outputs')

    # Parse arguments.
    args = p.parse_args()

    # Build kernel.
    m = platform.machine()

    if m == 'aarch64' or m == 'arm64':
        kern = cargo(
            'obkrnl',
            toolchain='nightly',
            target='aarch64-unknown-none-softfloat',
            release=args.release,
            args=['-Z', 'build-std=core,alloc'])
    elif m == 'x86_64' or m == 'AMD64':
        kern = cargo(
            'obkrnl',
            target='x86_64-unknown-none',
            release=args.release)
    else:
        print(f'Architecture {m} is not supported.', file=sys.stderr)
        sys.exit(1)

    # Build GUI.
    gui = cargo('gui', release=args.release, args=['--bin', 'obliteration', '-F', 'slint'])

    # Create output directory.
    dest = args.root

    if dest is None:
        dest = 'dist'

        if os.path.exists(dest):
            shutil.rmtree(dest)

        os.mkdir(dest)

    # Export artifacts.
    s = platform.system()

    if s == 'Darwin':
        export_darwin(dest, kern, gui)
    elif s == 'Linux':
        export_linux(dest, kern, gui)
    elif s == 'Windows':
        export_windows(dest, kern, gui)
    else:
        print(f'OS {s} is not supported.', file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()
