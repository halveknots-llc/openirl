<#
.SYNOPSIS
  OBS WebSocket v5 smoke tests for OpenIRL feature areas.

.DESCRIPTION
  Uses only built-in PowerShell/.NET APIs. Supports the OBS WebSocket Hello ->
  Identify -> Identified handshake, including OBS password authentication. Run
  status checks first, then scene checks. Stream-control tests should only run
  against a private OBS profile/channel.
#>
param(
  [ValidateSet('Status','Scenes','StreamControls','Production','All')]
  [string]$Action = 'Status',
  [string]$Uri = 'ws://127.0.0.1:4455',
  [string]$Password = $env:OPENIRL_OBS_PASSWORD,
  [string[]]$SceneNames = @('OpenIRL Live','OpenIRL BRB','OpenIRL Low Signal','OpenIRL Backup Feed','OpenIRL Privacy'),
  [switch]$DryRun,
  [string]$OutFile = 'artifacts/alpha/obs-smoke.json'
)

$ErrorActionPreference = 'Stop'

function ConvertTo-ObsAuthResponse {
  param([string]$Password, [string]$Salt, [string]$Challenge)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  $secretBytes = [System.Text.Encoding]::UTF8.GetBytes($Password + $Salt)
  $secret = [Convert]::ToBase64String($sha.ComputeHash($secretBytes))
  $responseBytes = [System.Text.Encoding]::UTF8.GetBytes($secret + $Challenge)
  return [Convert]::ToBase64String($sha.ComputeHash($responseBytes))
}

function Send-Json {
  param($Socket, $Object)
  $json = $Object | ConvertTo-Json -Depth 20 -Compress
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
  $segment = [ArraySegment[byte]]::new($bytes)
  $Socket.SendAsync($segment, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, [Threading.CancellationToken]::None).GetAwaiter().GetResult() | Out-Null
}

function Receive-Json {
  param($Socket)
  $buffer = New-Object byte[] 65536
  $builder = New-Object System.Text.StringBuilder
  do {
    $segment = [ArraySegment[byte]]::new($buffer)
    $result = $Socket.ReceiveAsync($segment, [Threading.CancellationToken]::None).GetAwaiter().GetResult()
    if ($result.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) {
      throw 'OBS WebSocket closed before a response was received.'
    }
    $chunk = [System.Text.Encoding]::UTF8.GetString($buffer, 0, $result.Count)
    [void]$builder.Append($chunk)
  } while (-not $result.EndOfMessage)
  return ($builder.ToString() | ConvertFrom-Json)
}

function Invoke-ObsRequest {
  param($Socket, [string]$RequestType, $RequestData = @{})
  $requestId = [Guid]::NewGuid().ToString()
  Send-Json $Socket @{ op = 6; d = @{ requestType = $RequestType; requestId = $requestId; requestData = $RequestData } }
  while ($true) {
    $packet = Receive-Json $Socket
    if ($packet.op -eq 7 -and $packet.d.requestId -eq $requestId) {
      return $packet.d
    }
  }
}

function Connect-Obs {
  param([string]$Uri, [string]$Password)
  $socket = [System.Net.WebSockets.ClientWebSocket]::new()
  $socket.ConnectAsync([Uri]$Uri, [Threading.CancellationToken]::None).GetAwaiter().GetResult()
  $hello = Receive-Json $socket
  if ($hello.op -ne 0) { throw "Expected Hello packet, got op=$($hello.op)" }
  $identify = @{ rpcVersion = 1 }
  if ($null -ne $hello.d.authentication) {
    if ([string]::IsNullOrWhiteSpace($Password)) { throw 'OBS requires authentication. Set OPENIRL_OBS_PASSWORD or pass -Password.' }
    $identify.authentication = ConvertTo-ObsAuthResponse -Password $Password -Salt $hello.d.authentication.salt -Challenge $hello.d.authentication.challenge
  }
  Send-Json $socket @{ op = 1; d = $identify }
  while ($true) {
    $packet = Receive-Json $socket
    if ($packet.op -eq 2) { return $socket }
  }
}

function Add-Result {
  param([System.Collections.ArrayList]$Results, [string]$Name, [bool]$Passed, $Details)
  [void]$Results.Add([ordered]@{ name = $Name; passed = $Passed; details = $Details })
}

New-Item -ItemType Directory -Force -Path (Split-Path $OutFile) | Out-Null
$results = [System.Collections.ArrayList]::new()
$socket = $null
try {
  $socket = Connect-Obs -Uri $Uri -Password $Password
  Add-Result $results 'connect' $true @{ uri = $Uri }

  if ($Action -in @('Status','All')) {
    Add-Result $results 'GetVersion' $true (Invoke-ObsRequest $socket 'GetVersion')
    Add-Result $results 'GetStreamStatus' $true (Invoke-ObsRequest $socket 'GetStreamStatus')
    Add-Result $results 'GetCurrentProgramScene' $true (Invoke-ObsRequest $socket 'GetCurrentProgramScene')
  }

  if ($Action -in @('Scenes','All')) {
    $sceneList = Invoke-ObsRequest $socket 'GetSceneList'
    Add-Result $results 'GetSceneList' $true $sceneList
    foreach ($sceneName in $SceneNames) {
      $response = Invoke-ObsRequest $socket 'SetCurrentProgramScene' @{ sceneName = $sceneName }
      Add-Result $results "SwitchScene:$sceneName" ($response.requestStatus.result -eq $true) $response
    }
  }

  if ($Action -in @('StreamControls','All')) {
    if ($DryRun) {
      Add-Result $results 'StreamControls' $true @{ dryRun = $true; note = 'Skipped StartStream/StopStream.' }
    } else {
      Add-Result $results 'StartStream' $true (Invoke-ObsRequest $socket 'StartStream')
      Start-Sleep -Seconds 2
      Add-Result $results 'StopStream' $true (Invoke-ObsRequest $socket 'StopStream')
    }
  }

  if ($Action -in @('Production','All')) {
    Add-Result $results 'SaveReplayBuffer' $true (Invoke-ObsRequest $socket 'SaveReplayBuffer')
    Add-Result $results 'StartRecord' $true (Invoke-ObsRequest $socket 'StartRecord')
    Start-Sleep -Seconds 2
    Add-Result $results 'StopRecord' $true (Invoke-ObsRequest $socket 'StopRecord')
  }
} catch {
  Add-Result $results 'error' $false @{ message = $_.Exception.Message }
} finally {
  if ($null -ne $socket) { $socket.Dispose() }
}

$report = [ordered]@{
  generated_at = (Get-Date).ToUniversalTime().ToString('o')
  action = $Action
  dry_run = [bool]$DryRun
  results = $results
  passed = -not ($results | Where-Object { -not $_.passed })
}
$report | ConvertTo-Json -Depth 30 | Set-Content -Encoding UTF8 $OutFile
$report | ConvertTo-Json -Depth 30
if (-not $report.passed) { exit 1 }
