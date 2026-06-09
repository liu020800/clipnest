param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$TauriArgs
)

$ErrorActionPreference = "Stop"

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$cargoExe = Join-Path $cargoBin "cargo.exe"

if (-not (Test-Path -LiteralPath $cargoExe)) {
  Write-Error "cargo.exe not found at $cargoExe. Install Rust with rustup before starting ClipNest."
}

$env:PATH = "$cargoBin;$env:PATH"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$tauriJs = Join-Path $repoRoot "node_modules\@tauri-apps\cli\tauri.js"

if (-not (Test-Path -LiteralPath $tauriJs)) {
  Write-Error "Tauri CLI not found at $tauriJs. Run npm install first."
}

& node $tauriJs @TauriArgs
exit $LASTEXITCODE
