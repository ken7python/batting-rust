$ErrorActionPreference = 'Stop'
$Target = 'x86_64-pc-windows-msvc'
$Root = Split-Path -Parent $PSScriptRoot

Push-Location $Root
try {
    $Metadata = cargo metadata --locked --format-version 1 --no-deps
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed.' }
    $Metadata = $Metadata | ConvertFrom-Json
    $Version = ($Metadata.packages | Where-Object { $_.name -eq 'batting' }).version
    cargo build --locked --release --target $Target
    if ($LASTEXITCODE -ne 0) { throw 'Windows build failed. Install the MSVC C++ tools, Windows SDK and Rust target.' }

    $Output = New-Item -ItemType Directory -Force -Path 'dist/windows'
    $Exe = Join-Path $Output.FullName 'Batting.exe'
    $License = Join-Path $Output.FullName 'OFL-NotoSansJP.txt'
    Copy-Item (Join-Path $Metadata.target_directory "$Target/release/batting.exe") $Exe
    Copy-Item 'assets/OFL-NotoSansJP.txt' $License
    $Archive = Join-Path $Output.FullName "batting-windows-x64-v$Version.zip"
    Compress-Archive -LiteralPath $Exe, $License -DestinationPath $Archive -Force
    Write-Output $Archive
}
finally {
    Pop-Location
}
