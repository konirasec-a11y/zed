# Run existing BDD integrations only, after committing and pushing the branch.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $root
try {
    & cargo +1.98.1 test -p bellatrix_mde --test bdd_native_class_authoring --test bdd_diagram_flows --test bdd_mdj_association_flows --test bdd_mdj_presentation_flows
    if ($LASTEXITCODE -ne 0) { throw 'MDE BDD integration flows failed' }
    & cargo +1.98.1 test -p bellatrix_mde_cordis --test bdd_mdj_edit_save_flows
    if ($LASTEXITCODE -ne 0) { throw 'Cordis BDD edit/save flow failed' }
    Write-Host 'Bellatrix BDD integrations completed. The removed CLI artifact smoke is retired; launch the supported application with cargo +1.98.1 run -p zed.'
}
finally {
    Pop-Location
}
