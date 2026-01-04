local function cargo(opts)
  local pkg = opts[1]

  -- Get toolchain selector.
  local tc = opts.toolchain

  if tc then
    tc = string.format('+%s', tc)
  end

  -- Get package ID.
  local id = os.capture('cargo', tc, 'pkgid', '-p', pkg)
  local url = Url:new(id)
  local path = url.path

  if os.kind == 'windows' then
    -- Remove '/' in front of drive letter.
    path = path:sub(2)
  end

  -- Setup build arguments.
  local args = {
    'build',
    '-p', pkg,
    '--message-format', 'json-render-diagnostics',
    table.unpack(opts.args or {})
  }

  if opts.release then
    args[#args + 1] = '-r'
  end

  -- Add target.
  local val = opts.target

  if val then
    args[#args + 1] = '--target'
    args[#args + 1] = val
  end

  -- Build.
  local proc <close> = os.spawn({'cargo', cwd = path, stdout = 'pipe'}, tc, table.unpack(args))
  local artifact = nil

  while true do
    -- Read JSON message.
    local line = proc.stdout:read()

    if not line then
      break
    end

    line = json.parse(line)

    -- Check type.
    local reason = line.reason

    if reason == 'build-finished' then
      if line.success then
        break
      else
        exit(1)
      end
    elseif reason == 'compiler-artifact' then
      if line.package_id == id then
        artifact = line
      end
    end
  end

  return artifact
end

-- Build kernel.
local arch = os.arch
local kern = nil

if arch == 'aarch64' then
  kern = cargo {
    'obkrnl',
    toolchain = 'nightly',
    target = 'aarch64-unknown-none-softfloat',
    release = args['release'],
    args = {'-Z', 'build-std=core,alloc'}
  }
elseif arch == 'x86_64' then
  kern = cargo {
    'obkrnl',
    target = 'x86_64-unknown-none',
    release = args['release']
  }
else
  error(string.format('architecture %s is not supported', arch))
end

-- Build GUI.
local gui = cargo {
  'gui',
  release = args['release'],
  args = {'--bin', 'obliteration'}
}

-- Create output directory.
local dest = args['root']

if not dest then
  dest = 'dist'

  os.removedir(dest)
  os.createdir(dest)
end

-- Export artifacts.
local kind = os.kind
local start = nil

if kind == 'linux' then
  -- Create directories.
  local bin = path.join(dest, 'bin')
  local share = path.join(dest, 'share')

  os.createdir(bin)
  os.createdir(share)

  -- Export files.
  start = path.join(bin, path.basename(gui.executable))

  os.copyfile(kern.executable, share)
  os.copyfileas(gui.executable, start, 'all')
elseif kind == 'macos' then
  -- Create directories.
  local bundle = path.join(dest, 'Obliteration.app')
  local contents = path.join(bundle, 'Contents')
  local macos = path.join(contents, 'MacOS')
  local resources = path.join(contents, 'Resources')

  os.createdir(bundle)
  os.createdir(contents)
  os.createdir(macos)
  os.createdir(resources)

  -- Export files.
  local name = path.basename(gui.executable)

  name = string.capitalize(name)
  start = path.join(macos, name)

  os.copyfile(kern.executable, resources)
  os.copyfileas(gui.executable, start, 'all')
  os.copyfileas('bundle.icns', path.join(resources, 'obliteration.icns'))
  os.copyfile('Info.plist', contents)

  -- Sign bundle.
  os.run('codesign', '-s', '-', '--entitlements', 'entitlements.plist', bundle)
elseif kind == 'windows' then
  -- Create share directory.
  local share = path.join(dest, 'share')

  os.createdir(share)

  -- Export files.
  start = path.join(dest, path.basename(gui.executable))

  os.copyfile(kern.executable, share)
  os.copyfileas(gui.executable, start)
else
  error(string.format('%s is not supported', kind))
end

-- Start VMM.
local addr = args['debug']

if addr then
  os.run(start, '--debug', addr)
end
