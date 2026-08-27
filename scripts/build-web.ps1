# Compila el build web y deja web/ listo para servir como estatico.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

cargo build -p vals-app --target wasm32-unknown-unknown --release
if ($LASTEXITCODE -ne 0) { throw "fallo la compilacion a wasm" }

Copy-Item "target/wasm32-unknown-unknown/release/vals-app.wasm" "web/vals.wasm" -Force

$kb = [math]::Round((Get-Item "web/vals.wasm").Length / 1KB, 1)
Write-Host "web/vals.wasm listo - $kb KB"
Write-Host "servir con: python -m http.server 8080 --directory web"
