#!/usr/bin/env python3
from argparse import ArgumentParser
import json
import os
import platform
import shutil
from subprocess import PIPE, Popen, run
import sys

def cargo(package, toolchain=None, target=None, release=False, args=None):
    # Get package ID.
    cmd = ['cargo']

    if toolchain is not None:
        cmd.append(f'+{toolchain}')

    id = run(cmd + ['pkgid', '-p', package], stdout=PIPE, check=True).stdout.decode('utf-8').strip()

    # Setup command and its argument.
    cmd.extend(['build', '-p', package])

    if target is not None:
        cmd.extend(['--target', target])

    if release:
        cmd.append('-r')

    if args is not None:
        cmd.extend(args)

    cmd.extend([
        '--message-format', 'json-render-diagnostics'
    ])

    # Run.
    with Popen(cmd, stdout=PIPE) as proc:
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

def export_darwin(root, kern):
    # Create bundle directory.
    bundle = os.path.join(root, 'Obliteration.app')

    os.mkdir(bundle)

    # Create Contents directory.
    contents = os.path.join(bundle, 'Contents')

    os.mkdir(contents)

    # Create Resources directory.
    resources = os.path.join(contents, 'Resources')

    os.mkdir(resources)

    # Copy kernel.
    shutil.copy(kern['executable'], resources)

def export_linux(root, kern):
    # Create share directory.
    share = os.path.join(root, 'share')

    os.mkdir(share)

    # Copy kernel.
    shutil.copy(kern['executable'], share)

def export_windows(root, kern):
    # Create share directory.
    share = os.path.join(root, 'share')

    os.mkdir(share)

    # Copy kernel.
    shutil.copy(kern['executable'], share)

def main():
    # Setup argument parser.
    p = ArgumentParser(
        description='Script to build Obliteration and create distribution file')

    p.add_argument('-r', '--release', action='store_true', help='enable optimization')

    # Parse arguments.
    args = p.parse_args()

    # Build kernel.
    m = platform.machine()

    if m == 'arm64' or m == 'aarch64':
        kern = cargo(
            'obkrnl',
            toolchain='nightly',
            target='aarch64-unknown-none-softfloat',
            release=args.release,
            args=['-Z', 'build-std=core,alloc'])
    elif m == 'x86_64':
        kern = cargo(
            'obkrnl',
            target='x86_64-unknown-none',
            release=args.release)
    else:
        print(f'Architecture {m} is not supported.', file=sys.stderr)
        sys.exit(1)

    # Create output directory.
    dest = 'dist'

    if os.path.exists(dest):
        shutil.rmtree(dest)

    os.mkdir(dest)

    # Export artifacts.
    s = platform.system()

    if s == 'Darwin':
        export_darwin(dest, kern)
    elif s == 'Linux':
        export_linux(dest, kern)
    elif s == 'Windows':
        export_windows(dest, kern)
    else:
        print(f'OS {s} is not supported.', file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()
