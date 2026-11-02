# Verify Bellatrix's bounded engines inside the Zed fork.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $root
try {
    & python script/bellatrix_mde/static_check.py
    if ($LASTEXITCODE -ne 0) { throw 'Bellatrix architecture check failed' }
    foreach ($name in @(
        'bellatrix_project', 'bellatrix_mde', 'bellatrix_mde_templates',
        'bellatrix_mde_cordis', 'bellatrix_mde_rust_generator',
        'bellatrix_mde_html_exporter', 'bellatrix_mde_workbench', 'bellatrix_zed_host'
    )) {
        & cargo +1.98.1 check -p $name
        if ($LASTEXITCODE -ne 0) { throw "Cargo check failed: $name" }
    }
    & cargo +1.98.1 test -p bellatrix_mde_cordis
    if ($LASTEXITCODE -ne 0) { throw 'MDE Cordis tests failed' }
}
finally {
    Pop-Location
}
