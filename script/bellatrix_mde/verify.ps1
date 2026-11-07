# Run only after committing and pushing the branch.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $root
try {
    & python script/bellatrix_mde/static_check.py
    if ($LASTEXITCODE -ne 0) { throw 'Bellatrix architecture check failed' }
    & cargo +1.98.1 check -p bellatrix_mde -p bellatrix_mde_cordis -p bellatrix_mde_ui -p bellatrix_zed_host
    if ($LASTEXITCODE -ne 0) { throw 'Bellatrix Cargo check failed' }
    & (Join-Path $PSScriptRoot 'smoke.ps1')
}
finally {
    Pop-Location
}
