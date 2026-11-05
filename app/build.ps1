[CmdletBinding()]
param(
    [string]$OutputDirectory = $PSScriptRoot,
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'
# A finished build always includes the current release and its portable package.
& (Join-Path $PSScriptRoot 'scripts/package-portable.ps1') -OutputDirectory $OutputDirectory -Offline:$Offline
