# Reproduce the documented legacy formatting using standard .NET APIs only.
# No original application code is loaded or executed.
$ErrorActionPreference = 'Stop'
$readings = @(0, 1, 49, 50, 51, 100, 950, 1000, 1050, 1150,
              2800, 2850, 2900, 2950, 3000, 4000, 5000, 8000, 10000, 12500)
$fixtures = @($readings | ForEach-Object {
    $mhz = $_
    $divided = [double][single]$mhz / 10000.0
    $rounded = [Math]::Round($divided, 2, [MidpointRounding]::ToEven)
    $scaled = $rounded * 100.0
    $digits = [Math]::Truncate($scaled).ToString('0', [Globalization.CultureInfo]::InvariantCulture)
    [ordered]@{
        MHz = $mhz
        Divided = $divided
        Rounded = $rounded
        Scaled = $scaled
        Text = $digits.PadLeft(2, '0').Insert(1, '.')
    }
})
$outputPath = Join-Path $PSScriptRoot 'cpu-frequency-dotnet-fixtures.json'
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText($outputPath, (ConvertTo-Json -InputObject $fixtures -Depth 4), $utf8)
Write-Output "Generated $($fixtures.Count) standard .NET CPU frequency fixtures."
