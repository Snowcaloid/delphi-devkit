param(
  [Parameter(Mandatory = $true)]
  [string]$ProjectPath,

  [Parameter(Mandatory = $true)]
  [string]$RSVarsPath,

  [Parameter(Mandatory = $true)]
  [string]$BuildArguments
)
function Test-FileLocked {
  param([string]$Path)
  if (-not (Test-Path $Path)) { return $false }
  try {
    $fs = [System.IO.File]::Open($Path, 'Open', 'ReadWrite', 'None')
    $fs.Close()
    return $false
  }
  catch {
    return $true
  }
}
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Write-Host "Setting up Delphi environment..." -ForegroundColor Cyan
$tempBatch = [System.IO.Path]::GetTempFileName() + ".bat"
try {
  @"
@echo off
call "$RSVarsPath"
set
"@ | Out-File -FilePath $tempBatch -Encoding ASCII

  $envOutput = & cmd.exe /c $tempBatch

  foreach ($line in $envOutput) {
    if ($line -match '^([^=]+)=(.*)$') {
      $varName = $matches[1]
      $varValue = $matches[2]

      # Skip system variables that shouldn't be changed
      if ($varName -notin @('PROCESSOR_ARCHITECTURE', 'PROCESSOR_IDENTIFIER', 'PROCESSOR_LEVEL', 'PROCESSOR_REVISION', 'NUMBER_OF_PROCESSORS')) {
        [Environment]::SetEnvironmentVariable($varName, $varValue, 'Process')
      }
    }
  }

  Write-Host "Environment variables updated from RSVars" -ForegroundColor Green
} finally {
  if (Test-Path $tempBatch) {
    Remove-Item $tempBatch -Force
  }
}
if (-not (Test-Path $ProjectPath)) {
  Write-Host "Error: Project file not found at $ProjectPath" -ForegroundColor Red
  exit 1
}
try {
  Write-Host "Building project: $ProjectPath" -ForegroundColor Cyan
  $projectDir = [IO.Path]::GetDirectoryName($ProjectPath)
  $projectBase = [IO.Path]::GetFileNameWithoutExtension($ProjectPath)
  $expectedExe = Join-Path $projectDir ("$projectBase.exe")
  if (Test-Path $expectedExe) {
    if (Test-FileLocked -Path $expectedExe) {
      Write-Host "Target executable appears to be in use: $expectedExe" -ForegroundColor Red
      Write-Host "Close the running application before compiling." -ForegroundColor Yellow
      Write-Host "(You can add logic to auto-terminate it if desired.)" -ForegroundColor DarkYellow
    }
  }
  $buildStart = Get-Date
  $buildArgsArray = $BuildArguments -split ' (?=(?:[^"]|"[^"]*")*$)' | Where-Object { $_ -ne '' }
  $allBuildArgs = @($ProjectPath) + $buildArgsArray
  & "msbuild.exe" @allBuildArgs | ForEach-Object {
    $_.Trim()
  }
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }
  if (Test-Path $expectedExe) {
    $exeInfo = Get-Item $expectedExe -ErrorAction SilentlyContinue
    if (-not $exeInfo -or $exeInfo.LastWriteTime -lt $buildStart) {
      exit 1
    }
  }
} catch {
  exit 1
}
