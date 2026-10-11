param()
# Build heart-portal as a static-PIE aarch64-musl binary with rcrt1.o startup.
# See README.md in this directory for the root-cause rationale.
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))  # scripts/android-pie -> repo root
Set-Location (Join-Path $repoRoot 'portal')

$env:CC_aarch64_unknown_linux_musl = "$env:LOCALAPPDATA\cargo-zigbuild\0.23.4\wrappers\cbb1\zigcc-aarch64-unknown-linux-musl-5e34.bat"
$env:CFLAGS_aarch64_unknown_linux_musl = '-fPIC'
$env:AR_aarch64_unknown_linux_musl  = "$env:LOCALAPPDATA\cargo-zigbuild\0.23.4\wrappers\cbb1\ar.exe"

cargo rustc --release --target aarch64-unknown-linux-musl -- `
  -C target-feature=+crt-static -C relocation-model=pic `
  -C linker="$PSScriptRoot\loglld.bat" -C linker-flavor=ld.lld -C link-self-contained=yes `
  -C link-arg=-pie -C debuginfo=0
exit $LASTEXITCODE
