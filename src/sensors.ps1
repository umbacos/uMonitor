param([switch]$DefinitionsOnly)
$ErrorActionPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$diskValues = @{}
$diskAt = [DateTime]::MinValue
$storageStatus = 'Windows storage: not sampled yet'
function Add-Sensor($values, [string]$provider, [string]$id, [string]$parent, [string]$name, [string]$type, $raw) {
    if ($null -eq $raw) { return }
    $n = 0.0
    if (-not [double]::TryParse(([string]$raw).Replace(',', '.'), [Globalization.NumberStyles]::Float, [Globalization.CultureInfo]::InvariantCulture, [ref]$n)) { return }
    if ([double]::IsNaN($n) -or [double]::IsInfinity($n) -or $n -lt 0) { return }
    $values[('sensor:' + $provider + ':' + $id)] = $n
    $context = $parent + ' ' + $id + ' ' + $name
    $cpu = $context -match '(intelcpu|amdcpu|/cpu/|\bCPU\b|Ryzen|Core i[3579]|Core Ultra)'
    if ($type -eq 'Temperature' -and $cpu -and $n -lt 150) {
        if (-not $values.ContainsKey('cpu.temperature') -or $name -match '(Package|Tctl|Tdie|Core Average)') { $values['cpu.temperature'] = $n }
    }
    if ($type -eq 'Power' -and $cpu -and $name -match '(Package|CPU Total|CPU Cores|PPT)') {
        if (-not $values.ContainsKey('cpu.power') -or $name -match 'Package|CPU Total|PPT') { $values['cpu.power'] = $n }
    }
    if ($type -eq 'Fan' -and ($cpu -or $name -match '(^|[ ._-])CPU([ ._-]|$)')) { $values['cpu.fan'] = $n }
    if ($type -eq 'Temperature' -and $context -match '(hdd|ssd|nvme|storage|S\.M\.A\.R\.T\.|drive)' -and $n -gt 0 -and $n -lt 150) {
        if (-not $values.ContainsKey('disk.temperature')) { $values['disk.temperature'] = $n }
    }
}
function Read-WebNode($node, [string]$parent, $values) {
    if ($null -eq $node) { return }
    $name = [string]$node.Text
    $context = $parent + '/' + $name
    if ($null -ne $node.Value -and [string]$node.Value -match '(-?\d+([.,]\d+)?)') {
        $raw = $Matches[1]
        $type = [string]$node.Type
        if (-not $type) {
            if ([string]$node.ImageURL -match 'temperature') { $type = 'Temperature' }
            elseif ([string]$node.ImageURL -match 'power') { $type = 'Power' }
            elseif ([string]$node.ImageURL -match 'fan') { $type = 'Fan' }
        }
        $id = [string]$node.SensorId
        if (-not $id) { $id = $context }
        Add-Sensor $values 'LocalMonitor' $id $parent $name $type $raw
    }
    foreach ($child in $node.Children) { Read-WebNode $child $context $values }
}
if ($DefinitionsOnly) { return }
while ($true) {
    $values = @{}
    $checked = New-Object System.Collections.Generic.List[string]
    foreach ($namespace in @('root/LibreHardwareMonitor', 'root/OpenHardwareMonitor')) {
        try {
            $sensors = @(Get-CimInstance -Namespace $namespace -ClassName Sensor -OperationTimeoutSec 2 -ErrorAction Stop)
            $checked.Add(($namespace.Split('/')[-1] + ' WMI: ' + $sensors.Count + ' sensors'))
            foreach ($sensor in $sensors) { Add-Sensor $values $namespace.Split('/')[-1] ([string]$sensor.Identifier) ([string]$sensor.Parent) ([string]$sensor.Name) ([string]$sensor.SensorType) $sensor.Value }
        } catch {
            $checked.Add(($namespace.Split('/')[-1] + ' WMI: unavailable'))
        }
    }
    # Monitor web servers work even when their optional WMI provider is disabled.
    try {
        $web = Invoke-RestMethod -Uri 'http://127.0.0.1:8085/data.json' -TimeoutSec 1 -ErrorAction Stop
        Read-WebNode $web '' $values
        $checked.Add('Local monitor web server: connected')
    } catch { $checked.Add('Local monitor web server: not running on port 8085') }
    # HWiNFO exposes selected gadget sensors here without needing its shared-memory SDK.
    $hwinfoRunning = @(Get-Process -Name 'HWiNFO64','HWiNFO32','HWiNFO' -ErrorAction SilentlyContinue).Count -gt 0
    $hwinfoCount = 0
    if ($hwinfoRunning) {
        foreach ($root in @('HKCU:', 'HKLM:')) {
            $registry = Get-ItemProperty -LiteralPath ($root + '\SOFTWARE\HWiNFO64\VSB') -ErrorAction SilentlyContinue
            if ($null -eq $registry) { continue }
            foreach ($property in $registry.PSObject.Properties) {
                if ($property.Name -notmatch '^ValueRaw(\d+)$') { continue }
                $index = $Matches[1]
                $name = [string]$registry.('Label' + $index)
                $parent = [string]$registry.('Sensor' + $index)
                $formatted = [string]$registry.('Value' + $index)
                $type = ''
                if ($formatted -match '(\u00B0C|\bC$)' -or $name -match 'Temperature|Tctl|Tdie') { $type = 'Temperature' }
                elseif ($formatted -match '\bRPM\b') { $type = 'Fan' }
                elseif ($formatted -match '\bW$') { $type = 'Power' }
                Add-Sensor $values 'HWiNFO' $index $parent $name $type $property.Value
                $hwinfoCount++
            }
        }
    }
    $checked.Add(('HWiNFO registry: ' + $hwinfoCount + ' sensors'))
    if (([DateTime]::UtcNow - $diskAt).TotalSeconds -ge 30) {
        $diskValues = @{}
        try {
            $disks = @(Get-PhysicalDisk -ErrorAction Stop | Sort-Object DeviceId)
            $systemDisk = (Get-Partition -DriveLetter C -ErrorAction SilentlyContinue | Get-Disk -ErrorAction SilentlyContinue | Select-Object -First 1).Number
            foreach ($disk in $disks) {
                $counter = $disk | Get-StorageReliabilityCounter -ErrorAction Stop
                if ($null -ne $counter.Temperature -and $counter.Temperature -gt 0 -and $counter.Temperature -lt 150) {
                    $diskValues[('sensor:Windows:disk/' + $disk.DeviceId + '/temperature')] = [double]$counter.Temperature
                    if (-not $diskValues.ContainsKey('disk.temperature') -or [string]$disk.DeviceId -eq [string]$systemDisk) { $diskValues['disk.temperature'] = [double]$counter.Temperature }
                }
            }
            $storageStatus = if ($diskValues.Count -gt 0) { 'Windows storage: temperature found' } else { 'Windows storage: driver returned no temperature' }
        } catch {
            $storageStatus = if ($_.Exception.Message -match 'denied|privilege|access|autoriz|negato') { 'Windows storage: sensor access denied' } else { 'Windows storage: reliability counters unavailable' }
        }
        $diskAt = [DateTime]::UtcNow
    }
    foreach ($key in $diskValues.Keys) { $values[$key] = $diskValues[$key] }
    $checked.Add($storageStatus)
    $values['_diagnostics'] = $checked -join '; '
    [Console]::WriteLine((ConvertTo-Json -InputObject $values -Compress))
    [Console]::Out.Flush()
    if ($env:COCKPIT_SENSOR_ONCE -eq '1') { break }
    Start-Sleep -Seconds 5
}
