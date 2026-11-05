[CmdletBinding()]
param(
    [string]$OutputDirectory,
    [switch]$SkipBuild,
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'
$appRoot = Split-Path $PSScriptRoot -Parent
if (-not $OutputDirectory) { $OutputDirectory = $appRoot }
$utf8 = New-Object System.Text.UTF8Encoding($false)
Push-Location -LiteralPath $appRoot
try {
    if (-not $SkipBuild) {
        $buildArgs = @('build', '--release', '--locked')
        if ($Offline) { $buildArgs += '--offline' }
        & cargo @buildArgs
        if ($LASTEXITCODE -ne 0) { throw 'The release build failed.' }
    }
    $metadataArgs = @('metadata', '--locked', '--format-version', '1', '--filter-platform', 'x86_64-pc-windows-msvc')
    if ($Offline) { $metadataArgs += '--offline' }
    $metadataText = & cargo @metadataArgs
    if ($LASTEXITCODE -ne 0) { throw 'Could not collect dependency metadata.' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
    $rootPackage = $metadata.packages | Where-Object { $_.manifest_path -eq (Join-Path $appRoot 'Cargo.toml') }
    if (-not $rootPackage) { throw 'Could not locate uMonitor package metadata.' }
    $version = $rootPackage.version
    $exe = Join-Path $metadata.target_directory 'release\uMonitor.exe'
    $sensorDll = Join-Path $metadata.target_directory 'release\msvcr120d.dll'
    if (-not (Test-Path -LiteralPath $exe)) { throw 'Missing release/uMonitor.exe.' }
    if (-not (Test-Path -LiteralPath $sensorDll)) { throw 'Missing bundled sensor library.' }

    # Each build uses a fresh staging directory; no previous or personal files are copied.
    $stageParent = Join-Path $metadata.target_directory ('portable-' + [guid]::NewGuid().ToString('N'))
    $stage = Join-Path $stageParent 'uMonitor'
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item -LiteralPath $exe,$sensorDll -Destination $stage
    foreach ($file in @('README.md', 'PORTABLE.md', 'THIRD_PARTY_NOTICES.md', 'MECHANICS.md')) {
        Copy-Item -LiteralPath (Join-Path $appRoot $file) -Destination $stage
    }
    # Include the entire tree, including nested assets and hidden config files.
    Copy-Item -LiteralPath (Join-Path $appRoot 'config') -Destination $stage -Recurse -Force
    $stageDocs = Join-Path $stage 'docs'
    New-Item -ItemType Directory -Path $stageDocs | Out-Null
    Copy-Item -LiteralPath (Join-Path $appRoot 'docs\screenshots') -Destination $stageDocs -Recurse

    $licenseDir = Join-Path $stage 'licenses'
    New-Item -ItemType Directory -Path $licenseDir | Out-Null
    $inventory = @()
    foreach ($package in $metadata.packages | Where-Object { $_.id -ne $rootPackage.id } | Sort-Object name,version) {
        $inventory += [pscustomobject]@{ name=$package.name; version=$package.version; license=$package.license; repository=$package.repository }
        $packageDir = Split-Path $package.manifest_path -Parent
        $notices = @(Get-ChildItem -LiteralPath $packageDir -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)' })
        $nested = @(Get-ChildItem -LiteralPath $packageDir -Directory | Where-Object { $_.Name -match '^licen[cs]es?$' })
        if ($notices.Count -gt 0 -or $nested.Count -gt 0) {
            $destination = Join-Path $licenseDir ($package.name + '-' + $package.version)
            New-Item -ItemType Directory -Path $destination | Out-Null
            foreach ($notice in $notices) { Copy-Item -LiteralPath $notice.FullName -Destination $destination }
            foreach ($folder in $nested) { Copy-Item -LiteralPath $folder.FullName -Destination $destination -Recurse }
        }
    }
    [IO.File]::WriteAllText((Join-Path $licenseDir 'dependencies.json'), (ConvertTo-Json -InputObject $inventory -Depth 5), $utf8)
    $entries = @(Get-ChildItem -LiteralPath $stage -File -Recurse -Force | Sort-Object FullName | ForEach-Object {
        [pscustomobject]@{ path=$_.FullName.Substring($stage.Length + 1).Replace('\','/'); bytes=$_.Length; sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    })
    $manifest = [ordered]@{ product='uMonitor'; version=$version; platform='windows-x86_64'; files=$entries }
    [IO.File]::WriteAllText((Join-Path $stage 'manifest.json'), ($manifest | ConvertTo-Json -Depth 6), $utf8)

    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
    $output = (Resolve-Path -LiteralPath $OutputDirectory).Path
    $zip = Join-Path $output ("uMonitor-$version-windows-x64-portable.zip")
    # Cargo source archives can contain pre-1980 dates that ZIP cannot represent.
    # Fixed staging dates also keep repeated packaging stable.
    $archiveTime = [datetime]'2000-01-01T00:00:00Z'
    Get-ChildItem -LiteralPath $stage -Recurse -Force | ForEach-Object { $_.LastWriteTimeUtc = $archiveTime }
    (Get-Item -LiteralPath $stage).LastWriteTimeUtc = $archiveTime
    # Compress-Archive skips hidden files. ZipFile includes them and empty folders.
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $pendingZip = Join-Path $output ([IO.Path]::GetRandomFileName() + '.zip')
    try {
        [IO.Compression.ZipFile]::CreateFromDirectory($stage, $pendingZip, [IO.Compression.CompressionLevel]::Optimal, $true)
        Move-Item -LiteralPath $pendingZip -Destination $zip -Force
    } finally {
        if (Test-Path -LiteralPath $pendingZip) { Remove-Item -LiteralPath $pendingZip -Force }
    }
    $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText(($zip + '.sha256'), ($hash + '  ' + (Split-Path $zip -Leaf) + "`n"), $utf8)
    Write-Output "Portable ZIP: $zip"
    Write-Output "Staged files: $stage"
    Write-Output "SHA-256: $hash"
} finally {
    Pop-Location
}
