# Smoke test for the OmniHub AirPlay add-on on Windows, outside MSYS2 so only
# the bundled DLLs are available:
#   1. uxplay.exe starts and prints its help (all DLLs resolve);
#   2. the receiver runs with the real video/audio sinks without missing elements;
#   3. it answers an AirPlay "GET /info" request on TCP 7000;
#   4. it announces itself over mDNS (_airplay._tcp) without Apple's Bonjour.
param([Parameter(Mandatory = $true)][string]$Zip)

$ErrorActionPreference = 'Stop'
$root = Join-Path $env:RUNNER_TEMP 'airplay-addon-test'
Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
Expand-Archive -Path $Zip -DestinationPath $root
$addon = Join-Path $root 'OmniHub-AirPlay'
$bin = Join-Path $addon 'bin'
$exe = Join-Path $bin 'uxplay.exe'

# Same environment OmniHub uses (crates/omnihub-core/src/capture/airplay.rs).
$env:PATH = "$bin;$env:SystemRoot\System32;$env:SystemRoot"
$env:GST_PLUGIN_SYSTEM_PATH_1_0 = Join-Path $addon 'lib\gstreamer-1.0'
$env:GST_PLUGIN_PATH_1_0 = ''
$env:GST_PLUGIN_SCANNER_1_0 = Join-Path $bin 'gst-plugin-scanner.exe'
$env:GST_REGISTRY_1_0 = Join-Path $root 'registry.bin'

# Let the receiver and this script's mDNS socket through the runner's firewall,
# as OmniHub's "Allow through the firewall" button does on a user's PC.
try {
  New-NetFirewallRule -DisplayName 'OmniHub AirPlay CI (uxplay)' -Direction Inbound -Program $exe -Action Allow | Out-Null
  New-NetFirewallRule -DisplayName 'OmniHub AirPlay CI (test)' -Direction Inbound -Program (Get-Process -Id $PID).Path -Action Allow | Out-Null
} catch {
  Write-Warning "Could not add firewall rules: $_"
}

function Start-Receiver([string[]]$ArgList, [string]$Name) {
  $out = Join-Path $root "$Name.out.txt"
  $err = Join-Path $root "$Name.err.txt"
  $p = Start-Process -FilePath $exe -ArgumentList $ArgList -RedirectStandardOutput $out -RedirectStandardError $err -PassThru -NoNewWindow
  return @{ Process = $p; Out = $out; Err = $err }
}

function Stop-Receiver($r) {
  if (-not $r.Process.HasExited) { Stop-Process -Id $r.Process.Id -Force }
  Start-Sleep -Milliseconds 500
  $text = (Get-Content $r.Out -Raw -ErrorAction SilentlyContinue) + (Get-Content $r.Err -Raw -ErrorAction SilentlyContinue)
  Write-Host "----- receiver output -----"
  Write-Host $text
  return $text
}

Write-Host '== 1. help'
$help = (& $exe -h 2>&1 | Out-String)
$code = $LASTEXITCODE
$help.Split("`n") | Select-Object -First 3 | Write-Host
# A missing DLL shows up as a negative NTSTATUS exit code (e.g. 0xC0000135).
if ($code -lt 0 -or $code -gt 1 -or $help -notmatch 'AirPlay') { throw "uxplay -h failed (exit code $code)" }

Write-Host '== 2. real sinks'
$r = Start-Receiver @('-n', 'OmniHubSinkTest', '-nh', '-p', '-vs', 'd3d11videosink', '-as', 'wasapi2sink') 'sinks'
Start-Sleep -Seconds 8
$text = Stop-Receiver $r
if ($text -match 'no element|could not link|erroneous pipeline|missing plugin') { throw 'GStreamer elements are missing from the add-on' }

Write-Host '== 3 + 4. AirPlay server and mDNS'
$r = Start-Receiver @('-n', 'OmniHubCI', '-nh', '-p', '-vs', 'fakesink', '-as', 'fakesink') 'server'
Start-Sleep -Seconds 6
try {
  if ($r.Process.HasExited) { throw "uxplay exited early with code $($r.Process.ExitCode)" }

  $client = New-Object System.Net.Sockets.TcpClient
  $client.Connect('127.0.0.1', 7000)
  $stream = $client.GetStream()
  $req = [Text.Encoding]::ASCII.GetBytes("GET /info RTSP/1.0`r`nCSeq: 1`r`nUser-Agent: AirPlay/690.7.1`r`nX-Apple-ProtocolVersion: 1`r`n`r`n")
  $stream.Write($req, 0, $req.Length)
  $stream.ReadTimeout = 5000
  $buf = New-Object byte[] 8192
  $n = $stream.Read($buf, 0, $buf.Length)
  $head = [Text.Encoding]::ASCII.GetString($buf, 0, [Math]::Min($n, 200))
  $client.Close()
  Write-Host "GET /info -> $($head.Split("`n")[0])"
  if (-not $head.StartsWith('RTSP/1.0 200')) { throw "unexpected /info response: $head" }

  # mDNS query for _airplay._tcp.local PTR with the unicast-response bit set.
  $q = [Collections.Generic.List[byte]]::new()
  $q.AddRange([byte[]](0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0))
  foreach ($label in '_airplay', '_tcp', 'local') {
    $q.Add([byte]$label.Length)
    $q.AddRange([Text.Encoding]::ASCII.GetBytes($label))
  }
  $q.AddRange([byte[]](0, 0, 12, 0x80, 1))
  $udp = New-Object System.Net.Sockets.UdpClient(0)
  $udp.Client.ReceiveTimeout = 2000
  $found = $false
  for ($i = 0; $i -lt 5 -and -not $found; $i++) {
    [void]$udp.Send($q.ToArray(), $q.Count, '224.0.0.251', 5353)
    $deadline = (Get-Date).AddSeconds(2)
    while ((Get-Date) -lt $deadline -and -not $found) {
      try {
        $from = New-Object System.Net.IPEndPoint([Net.IPAddress]::Any, 0)
        $resp = $udp.Receive([ref]$from)
        if ([Text.Encoding]::ASCII.GetString($resp).Contains('OmniHubCI')) { $found = $true }
      } catch [System.Net.Sockets.SocketException] { break }
    }
  }
  $udp.Close()
  Write-Host "mDNS announcement found: $found"
  if (-not $found) { throw 'the receiver did not answer the _airplay._tcp mDNS query' }
} finally {
  $text = Stop-Receiver $r
}
Write-Host 'AirPlay add-on smoke test passed'
