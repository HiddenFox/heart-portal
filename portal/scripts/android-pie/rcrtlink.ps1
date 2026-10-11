param([string]$RespFile)
# Re-link with rcrt1.o (self-relocating startup) instead of crt1.o.
$lld = 'C:\Users\feng.han\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe'
$c = [System.IO.File]::ReadAllText($RespFile)
[System.IO.File]::WriteAllText("$PSScriptRoot\linker-args-captured.txt", $c)
$n = $c.Replace('self-contained\crt1.o', 'self-contained\rcrt1.o')
$n = [regex]::Replace($n, 'aarch64-unknown-linux-musl\\release\\deps\\heart_portal-[0-9a-f]+"', 'aarch64-unknown-linux-musl\\release\\heart-portal-pie"')
[System.IO.File]::WriteAllText("$PSScriptRoot\linker-args-rcrt.txt", $n)
& $lld -flavor gnu "@$PSScriptRoot\linker-args-rcrt.txt"
'RCRT_LINK_EXIT=' + $LASTEXITCODE
