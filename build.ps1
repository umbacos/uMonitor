[CmdletBinding()]
param(
    [string]$OutputDirectory,
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'
if (-not $OutputDirectory) {
    $projectRoot = Split-Path $PSScriptRoot -Parent
    if (Test-Path -LiteralPath (Join-Path $projectRoot '.git')) {
        $OutputDirectory = Join-Path $projectRoot 'dist'
    } else {
        $OutputDirectory = Join-Path $PSScriptRoot 'dist'
    }
}
# A finished build always includes the current release and its portable package.
& (Join-Path $PSScriptRoot 'scripts/package-portable.ps1') -OutputDirectory $OutputDirectory -Offline:$Offline
if ($LASTEXITCODE -ne 0) { throw 'Portable packaging failed.' }
& python (Join-Path $PSScriptRoot 'scripts/package-release.py') --output $OutputDirectory
if ($LASTEXITCODE -ne 0) { throw 'Release packaging failed.' }
