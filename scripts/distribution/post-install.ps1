# Generate the offline API documentation for the installed targets.
$ErrorActionPreference = 'Stop'
$scoopRoot = $PSScriptRoot
$scoopPlan = Join-Path $scoopRoot 'share/scoop/install'
$scoopTargets = @()
$scoopAllTargets = $false
$scoopJobs = 6
$scoopWork = Join-Path $scoopRoot 'build/post-install'

for ($scoopIndex = 0; $scoopIndex -lt $args.Count; $scoopIndex++) {
    $scoopOption = $args[$scoopIndex]
    switch ($scoopOption) {
        { $_ -in '--target', '--jobs', '--work-dir' } {
            $scoopIndex++
            if ($scoopIndex -ge $args.Count -or [string]::IsNullOrEmpty($args[$scoopIndex])) {
                throw "$scoopOption requires a value"
            }
            $scoopValue = $args[$scoopIndex]
            switch ($scoopOption) {
                '--target' {
                    if ($scoopValue -cnotin $scoopTargets) { $scoopTargets += $scoopValue }
                }
                '--jobs' {
                    if (-not [int]::TryParse($scoopValue, [ref]$scoopJobs) -or $scoopJobs -lt 1) {
                        throw '--jobs must be positive'
                    }
                }
                '--work-dir' { $scoopWork = $scoopValue }
            }
        }
        '--all-targets' { $scoopAllTargets = $true }
        { $_ -in '--help', '-h' } {
            Write-Output @'
Usage: & ./post-install.ps1 [options]
Generate offline API documentation for the installed targets.

  --target TARGET        Document an installed target (repeatable)
  --all-targets          Document every installed target of this release
  --jobs N               Maximum parallel build jobs (default: 6)
  --work-dir PATH        Build cache and intermediates (default: build/post-install)

With no target option, document the release's default target.
'@
            exit 0
        }
        default { throw "unknown option: $scoopOption (see --help)" }
    }
}

$scoopAvailable = @(Get-Content -LiteralPath "$scoopPlan/targets.list")
$scoopVersion = (Get-Content -LiteralPath "$scoopRoot/VERSION" -TotalCount 1)
function ScoopInstalled([string]$Target) {
    return Test-Path -LiteralPath "$scoopRoot/lib/$Target/scoop.core.slib"
}
if ($scoopAllTargets) {
    if ($scoopTargets.Count -ne 0) { throw '--target and --all-targets are mutually exclusive' }
    $scoopTargets = @($scoopAvailable | Where-Object { ScoopInstalled $_ })
    if ($scoopTargets.Count -eq 0) {
        throw "no target package is installed; extract a scoop-$scoopVersion-lib-<target>.tar.xz archive first"
    }
} elseif ($scoopTargets.Count -eq 0) {
    $scoopTargets = @($scoopAvailable[0])
}
foreach ($scoopTarget in $scoopTargets) {
    if ($scoopTarget -cnotin $scoopAvailable) {
        throw "target $scoopTarget is not provided by this release"
    }
    if (-not (ScoopInstalled $scoopTarget)) {
        throw "target $scoopTarget is not installed: extract scoop-$scoopVersion-lib-$scoopTarget.tar.xz into $scoopRoot first"
    }
}

if (-not [IO.Path]::IsPathRooted($scoopWork)) {
    $scoopWork = Join-Path (Get-Location).Path $scoopWork
}
[IO.Directory]::CreateDirectory($scoopWork) | Out-Null
$scoopWork = (Resolve-Path -LiteralPath $scoopWork).Path
$scoopScratch = Join-Path $scoopWork 'scratch'
[IO.Directory]::CreateDirectory($scoopScratch) | Out-Null
$scoopPreviousTmpdir = $env:TMPDIR
$scoopPreviousTmp = $env:TMP
$scoopPreviousTemp = $env:TEMP
try {
    $env:TMPDIR = $scoopScratch
    $env:TMP = $scoopScratch
    $env:TEMP = $scoopScratch
    foreach ($scoopTarget in $scoopTargets) {
        $scoopCones = @(Get-Content -LiteralPath "$scoopPlan/$scoopTarget.list")
        Write-Output "Generating API documentation for $scoopTarget"
        $scoopCommand = @('doc') + @(
            foreach ($scoopCone in $scoopCones) { "$scoopRoot/src/$scoopCone/Cone.toml" }
        ) + @(
            '--jobs', $scoopJobs, '--target', $scoopTarget,
            '--sysroot', $scoopRoot, '--scoopc', "$scoopRoot/bin/scoopc.exe",
            '--no-deps', '--deny-doc-warnings', '--cache-dir', "$scoopWork/cache",
            '--target-dir', "$scoopWork/doc-target", '--output-dir', "$scoopRoot/docs/$scoopTarget"
        )
        & "$scoopRoot/bin/sco.exe" @scoopCommand
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    $scoopLinks = foreach ($scoopTarget in $scoopAvailable) {
        if (Test-Path -LiteralPath "$scoopRoot/docs/$scoopTarget/index.html") {
            '<li><a href="{0}/index.html">{0}</a></li>' -f $scoopTarget
        }
    }
    $scoopHtml = @'
<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Scoop built-in API documentation</title></head>
<body><main><h1>Scoop built-in API documentation</h1>
<p>Select the target used to compile your application.
All pages, source text, and search resources are included locally.</p><ul>
'@ + "`n" + ($scoopLinks -join "`n") + "`n</ul></main></body></html>`n"
    Set-Content -LiteralPath "$scoopRoot/docs/index.html" -Value $scoopHtml -Encoding UTF8
    Write-Output "API documentation is ready in $scoopRoot"
} finally {
    $env:TMPDIR = $scoopPreviousTmpdir
    $env:TMP = $scoopPreviousTmp
    $env:TEMP = $scoopPreviousTemp
}
