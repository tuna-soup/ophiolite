$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot

try {
    cargo run -p contracts-export
}
finally {
    Pop-Location
}
