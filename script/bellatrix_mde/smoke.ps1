# Test Bellatrix MDE end-to-end from the Zed fork root.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $root
try {
    $fixture = 'crates/bellatrix_mde_model/fixtures/todo.json'
    function Run-Cargo {
        param([string[]]$CargoArgs)
        & cargo +1.98.1 @CargoArgs
        if ($LASTEXITCODE -ne 0) { throw "Cargo command failed: $($CargoArgs -join ' ')" }
    }
    Run-Cargo -CargoArgs @('test', '-p', 'bellatrix_mde_cordis')
    Run-Cargo -CargoArgs @('run', '-p', 'bellatrix_mde_cli', '--', 'check', $fixture)
    Run-Cargo -CargoArgs @('run', '-p', 'bellatrix_mde_cli', '--', 'analyze', $fixture)
    $output = Join-Path $env:TEMP ("bellatrix-mde-" + [Guid]::NewGuid().ToString('N'))
    Run-Cargo -CargoArgs @('run', '-p', 'bellatrix_mde_cli', '--', 'generate', $fixture, '--output', (Join-Path $output 'rust'))
    Run-Cargo -CargoArgs @('check', '--manifest-path', (Join-Path $output 'rust/Cargo.toml'))
    Run-Cargo -CargoArgs @('run', '-p', 'bellatrix_mde_cli', '--', 'export', $fixture, '--output', (Join-Path $output 'html'))
    if (-not (Test-Path (Join-Path $output 'html/index.html'))) {
        throw 'MDE HTML index was not generated'
    }
    Write-Host "Bellatrix MDE smoke artifacts: $output"
}
finally {
    Pop-Location
}
