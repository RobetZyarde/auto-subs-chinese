param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $CargoArgs
)

$ErrorActionPreference = "Continue"

$inWarningBlock = $false
$skipNextBlank = $false

function Test-WarningStart {
  param([string] $Line)
  return $Line -match '^\s*warning(:|\[|$)' -or
    $Line -match '^\s*= note: `#\[warn\(' -or
    $Line -match '^\s*= help:' -or
    $Line -match '^\s*help:' -or
    $Line -match '^\s*\(!\)' -or
    $Line -match '^\s*CMake (Deprecation )?Warning' -or
    $Line -match '\bwarning C\d{4}:'
}

function Test-WarningContinuation {
  param([string] $Line)
  return $Line -match '^\s*(\||=|-->|::|\d+\s*\||\^\s*$)' -or
    $Line -match '^\s*(the template instantiation context|see reference to function template instantiation)' -or
    $Line -match '^\s*\(compiling source file ' -or
    $Line -match '^\s*For compatibility with older versions of CMake' -or
    $Line -match '^\s*Update the VERSION argument' -or
    $Line -match '^\s*Manually-specified variables were not used by the project:' -or
    $Line -match '^\s*CMAKE_' -or
    $Line -match '^\s*This warning is for project developers\.' -or
    $Line -match '^\s*Use -Wno-dev to suppress it\.' -or
    $Line -match '^\s*Call Stack \(most recent call first\):' -or
    $Line -match '^\s*Called from:' -or
    $Line -match '^\s*Returning to\s+' -or
    $Line -match '^\s*Entering\s+' -or
    $Line -match '^\s*$'
}

& cargo @CargoArgs 2>&1 | ForEach-Object {
  if ($_ -is [System.Management.Automation.ErrorRecord]) {
    $line = $_.Exception.Message
  } else {
    $line = $_.ToString()
  }

  if (Test-WarningStart $line) {
    $inWarningBlock = $true
    $skipNextBlank = $true
    return
  }

  if ($inWarningBlock -and (Test-WarningContinuation $line)) {
    return
  }

  if ($skipNextBlank -and $line -match '^\s*$') {
    $skipNextBlank = $false
    return
  }

  $inWarningBlock = $false
  $skipNextBlank = $false
  [Console]::Out.WriteLine($line)
}

exit $LASTEXITCODE
