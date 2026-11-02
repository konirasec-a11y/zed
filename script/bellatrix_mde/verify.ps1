# Compatibility redirect: Engine-MDE is part of the Bellatrix root workspace.
$ErrorActionPreference = 'Stop'
& (Join-Path $PSScriptRoot '../../../scripts/verify-project.ps1')
if ($LASTEXITCODE -ne 0) { throw 'Bellatrix project verification failed' }
