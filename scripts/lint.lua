-- Get kernel target.
local arch = os.arch
local kt = nil

if arch == 'aarch64' then
  kt = 'aarch64-unknown-none-softfloat'
elseif arch == 'x86_64' then
  kt = 'x86_64-unknown-none'
else
  error(string.format('architecture %s is not supported', arch))
end

-- Setup list of packages to lint.
local pkgs = {
  {name = 'bitflag', target = kt},
  {name = 'config', target = kt, feats = {'virt'}},
  {name = 'krt', target = kt},
  {name = 'macros'}
}

if arch == 'aarch64' then
  pkgs[#pkgs + 1] = {name = 'aarch64', target = 'aarch64-unknown-none-softfloat'}
elseif arch == 'x86_64' then
  pkgs[#pkgs + 1] = {name = 'hv'}
  pkgs[#pkgs + 1] = {name = 'obkrnl', target = 'x86_64-unknown-none'}
  pkgs[#pkgs + 1] = {name = 'x86-64', target = 'x86_64-unknown-none'}
end

for i = 1, #pkgs do
  -- Build arguments.
  local pkg = pkgs[i]
  local args = {'clippy', '--package', pkg.name, '--no-deps'}
  local val = pkg.target

  if val then
    args[#args + 1] = '--target'
    args[#args + 1] = val
  end

  val = pkg.feats

  if val then
    args[#args + 1] = '--features'
    args[#args + 1] = table.concat(val, ',')
  end

  args[#args + 1] = '--'
  args[#args + 1] = '-D'
  args[#args + 1] = 'warnings'

  -- Run clippy.
  os.run('cargo', table.unpack(args))
end
