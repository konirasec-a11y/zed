# Run from anywhere in the integrated Bellatrix source tree.
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
Set-Location $root
function Run-Cargo { param([string[]]$Args)
    & cargo @Args
    if ($LASTEXITCODE -ne 0) { throw "cargo $($Args -join ' ') failed" }
}
Run-Cargo -Args @('test','-p','bellatrix-mde-cordis')
Run-Cargo -Args @('run','-p','bellatrix-mde-cli','--','check','engines/mde/fixtures/todo.json')
Run-Cargo -Args @('run','-p','bellatrix-mde-cli','--','analyze','engines/mde/fixtures/todo.json')
$out = Join-Path $env:TEMP ("bellatrix-smoke-" + [Guid]::NewGuid().ToString('N'))
Run-Cargo -Args @('run','-p','bellatrix-mde-cli','--','generate','engines/mde/fixtures/todo.json','--output',(Join-Path $out 'todo'))
Run-Cargo -Args @('check','--manifest-path',(Join-Path $out 'todo/Cargo.toml'))
Run-Cargo -Args @('run','-p','bellatrix-mde-cli','--','export','engines/mde/fixtures/todo.json','--output',(Join-Path $out 'html'))
if (!(Test-Path (Join-Path $out 'html/index.html'))) { throw 'HTML index not emitted' }
Write-Host "Engine-MDE smoke generated output: $out"
