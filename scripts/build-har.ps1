param(
  [switch]$SkipVsDevCmd
)

$ErrorActionPreference = 'Stop'

$TargetTriple = 'aarch64-unknown-linux-ohos'
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

function Find-OhosNdkHome {
  if ($env:OHOS_NDK_HOME) {
    return $env:OHOS_NDK_HOME
  }
  if ($env:OHOS_SDK_HOME) {
    $candidate = Join-Path $env:OHOS_SDK_HOME 'default\openharmony'
    if (Test-Path -LiteralPath (Join-Path $candidate 'native\llvm\bin')) {
      return $candidate
    }
  }
  if ($env:DEVECO_SDK_HOME) {
    $candidate = Join-Path $env:DEVECO_SDK_HOME 'default\openharmony'
    if (Test-Path -LiteralPath (Join-Path $candidate 'native\llvm\bin')) {
      return $candidate
    }
  }

  $candidates = @(
    "$env:USERPROFILE\Huawei\Sdk\default\openharmony",
    "$env:LOCALAPPDATA\Huawei\Sdk\default\openharmony",
    'C:\Program Files\Huawei\DevEco Studio\sdk\default\openharmony'
  )

  foreach ($candidate in $candidates) {
    if (Test-Path -LiteralPath (Join-Path $candidate 'native\llvm\bin')) {
      return $candidate
    }
  }

  throw 'OHOS_NDK_HOME is not set and no default HarmonyOS/OpenHarmony SDK was found.'
}

function Import-VsDevCmd {
  if ($SkipVsDevCmd -or (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    return
  }

  $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
  if (-not (Test-Path -LiteralPath $vswhere)) {
    throw 'Visual Studio Build Tools were not found. Install the C++ build tools workload or run from a Developer PowerShell.'
  }

  $installPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  if (-not $installPath) {
    throw 'Visual Studio C++ build tools were not found. Install Microsoft.VisualStudio.Component.VC.Tools.x86.x64.'
  }

  $vsDevCmd = Join-Path $installPath 'Common7\Tools\VsDevCmd.bat'
  if (-not (Test-Path -LiteralPath $vsDevCmd)) {
    throw "VsDevCmd.bat was not found under $installPath."
  }

  $envOutput = cmd /s /c "`"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && set"
  foreach ($line in $envOutput) {
    $index = $line.IndexOf('=')
    if ($index -gt 0) {
      $name = $line.Substring(0, $index)
      $value = $line.Substring($index + 1)
      Set-Item -LiteralPath "Env:$name" -Value $value
    }
  }
}

function Find-Tool($Name) {
  foreach ($suffix in @('.exe', '.cmd', '')) {
    $path = Join-Path $script:ToolchainBin "$Name$suffix"
    if (Test-Path -LiteralPath $path) {
      return $path
    }
  }
  throw "Required OHOS tool was not found: $Name"
}

function Convert-ToNoSpacePath($Path) {
  $resolved = (Resolve-Path -LiteralPath $Path).Path
  if (-not $resolved.Contains(' ')) {
    return $resolved
  }

  $shortPath = (& cmd /d /c "for %I in (`"$resolved`") do @echo %~sI" | Select-Object -First 1)
  if ($shortPath -and -not $shortPath.Contains(' ') -and (Test-Path -LiteralPath $shortPath)) {
    return $shortPath
  }

  return $resolved
}

function Convert-ToShellPath($Path) {
  $noSpacePath = Convert-ToNoSpacePath $Path
  return $noSpacePath.Replace('\', '/')
}

function Remove-PathInside($Root, $Path) {
  if (-not (Test-Path -LiteralPath $Path)) {
    return
  }

  $resolvedRoot = (Resolve-Path -LiteralPath $Root).Path
  $resolvedPath = (Resolve-Path -LiteralPath $Path).Path
  $comparison = [StringComparison]::OrdinalIgnoreCase
  $rootWithSeparator = $resolvedRoot.TrimEnd('\') + '\'
  if (-not $resolvedPath.StartsWith($rootWithSeparator, $comparison)) {
    throw "Refusing to remove path outside ${resolvedRoot}: $resolvedPath"
  }

  Remove-Item -LiteralPath $resolvedPath -Recurse -Force
}

function Clear-NativeOutputs {
  param(
    [switch]$IncludeTarget
  )

  $archDirs = @('arm64-v8a', 'armeabi-v7a', 'x86', 'x86_64')
  $roots = @(
    (Join-Path $RepoRoot 'dist'),
    (Join-Path $RepoRoot 'package\libs')
  )

  foreach ($root in $roots) {
    if (-not (Test-Path -LiteralPath $root)) {
      continue
    }

    foreach ($arch in $archDirs) {
      if ($IncludeTarget -or $arch -ne 'arm64-v8a') {
        Remove-PathInside $root (Join-Path $root $arch)
      }
    }
  }
}

Import-VsDevCmd

$defaultMakeJobs = [Math]::Max(2, [Math]::Min([Environment]::ProcessorCount, 8))
$env:CARGO_BUILD_JOBS = if ($env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS } else { "$defaultMakeJobs" }
$env:MAKEFLAGS = if ($env:MAKEFLAGS) { $env:MAKEFLAGS } else { "-j$defaultMakeJobs" }
$env:CMAKE_BUILD_PARALLEL_LEVEL = if ($env:CMAKE_BUILD_PARALLEL_LEVEL) { $env:CMAKE_BUILD_PARALLEL_LEVEL } else { "$defaultMakeJobs" }

$env:OHOS_NDK_HOME = (Resolve-Path (Find-OhosNdkHome)).Path
$script:ToolchainBin = Join-Path $env:OHOS_NDK_HOME 'native\llvm\bin'
$sysroot = Convert-ToShellPath (Join-Path $env:OHOS_NDK_HOME 'native\sysroot')

if (-not (Test-Path -LiteralPath $script:ToolchainBin)) {
  throw "OHOS_NDK_HOME does not contain native\llvm\bin: $env:OHOS_NDK_HOME"
}

$clang = Convert-ToShellPath (Find-Tool 'clang')
$clangxx = Convert-ToShellPath (Find-Tool 'clang++')
$ar = Convert-ToShellPath (Find-Tool 'llvm-ar')
$ranlib = Convert-ToShellPath (Find-Tool 'llvm-ranlib')
$ohosClangTarget = 'aarch64-linux-ohos'
$commonFlags = "--target=$ohosClangTarget --sysroot=`"$sysroot`" -D__MUSL__"
$rustFlags = @("-Clink-arg=--target=$ohosClangTarget", "-Clink-arg=--sysroot=$sysroot")

$env:CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER = $clang
$env:CC_aarch64_unknown_linux_ohos = $clang
$env:CXX_aarch64_unknown_linux_ohos = $clangxx
$env:AR_aarch64_unknown_linux_ohos = $ar
$env:RANLIB_aarch64_unknown_linux_ohos = $ranlib
$env:LIBCLANG_PATH = if ($env:LIBCLANG_PATH) { $env:LIBCLANG_PATH } else { Join-Path $env:OHOS_NDK_HOME 'native\llvm\bin' }
$env:CFLAGS_aarch64_unknown_linux_ohos = if ($env:CFLAGS_aarch64_unknown_linux_ohos) { $env:CFLAGS_aarch64_unknown_linux_ohos } else { $commonFlags }
$env:CXXFLAGS_aarch64_unknown_linux_ohos = if ($env:CXXFLAGS_aarch64_unknown_linux_ohos) { $env:CXXFLAGS_aarch64_unknown_linux_ohos } else { $commonFlags }
$env:BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_ohos = if ($env:BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_ohos) { $env:BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_ohos } else { $commonFlags }
$env:PKG_CONFIG_ALLOW_CROSS = if ($env:PKG_CONFIG_ALLOW_CROSS) { $env:PKG_CONFIG_ALLOW_CROSS } else { '1' }
$env:CC_SHELL_ESCAPED_FLAGS = if ($env:CC_SHELL_ESCAPED_FLAGS) { $env:CC_SHELL_ESCAPED_FLAGS } else { '1' }
$env:VCPKG_ROOT = if ($env:VCPKG_ROOT) {
  $env:VCPKG_ROOT
} else {
  Join-Path $RepoRoot 'target\ohos-vcpkg'
}
$env:VCPKG_INSTALLED_ROOT = if ($env:VCPKG_INSTALLED_ROOT) {
  $env:VCPKG_INSTALLED_ROOT
} else {
  Join-Path $env:VCPKG_ROOT 'installed'
}
$env:OHOS_VCPKG_PREFIX = if ($env:OHOS_VCPKG_PREFIX) {
  $env:OHOS_VCPKG_PREFIX
} else {
  Join-Path $env:VCPKG_INSTALLED_ROOT 'arm64-linux'
}
$env:OHOS_LIBVPX_ROOT = if ($env:OHOS_LIBVPX_ROOT) {
  $env:OHOS_LIBVPX_ROOT
} else {
  $env:OHOS_VCPKG_PREFIX
}
$env:OHOS_LIBAOM_ROOT = if ($env:OHOS_LIBAOM_ROOT) {
  $env:OHOS_LIBAOM_ROOT
} else {
  $env:OHOS_VCPKG_PREFIX
}
$env:OHOS_LIBYUV_ROOT = if ($env:OHOS_LIBYUV_ROOT) { $env:OHOS_LIBYUV_ROOT } else { $env:OHOS_VCPKG_PREFIX }
$env:OHOS_LIBOPUS_ROOT = if ($env:OHOS_LIBOPUS_ROOT) { $env:OHOS_LIBOPUS_ROOT } else { $env:OHOS_VCPKG_PREFIX }

$rustFlagsSeparator = [string][char]0x1f
$encodedRustFlags = $rustFlags -join $rustFlagsSeparator
$env:CARGO_ENCODED_RUSTFLAGS = if ($env:CARGO_ENCODED_RUSTFLAGS) {
  "$env:CARGO_ENCODED_RUSTFLAGS$rustFlagsSeparator$encodedRustFlags"
} else {
  $encodedRustFlags
}

if (-not (Get-Command ohrs -ErrorAction SilentlyContinue)) {
  throw 'ohrs was not found in PATH. Install it with: cargo install ohrs --locked'
}

$bash = Get-Command bash -ErrorAction SilentlyContinue
if (-not $bash) {
  throw 'bash was not found. Git for Windows is required to build the pinned upstream codec dependencies.'
}
& $bash.Source (Join-Path $RepoRoot 'scripts\build-libvpx-ohos.sh')
if ($LASTEXITCODE -ne 0) {
  throw "OHOS libvpx build failed with exit code $LASTEXITCODE."
}
& $bash.Source (Join-Path $RepoRoot 'scripts\build-libaom-ohos.sh')
if ($LASTEXITCODE -ne 0) {
  throw "OHOS libaom build failed with exit code $LASTEXITCODE."
}
& $bash.Source (Join-Path $RepoRoot 'scripts\build-libyuv-ohos.sh')
if ($LASTEXITCODE -ne 0) {
  throw "OHOS libyuv build failed with exit code $LASTEXITCODE."
}
& $bash.Source (Join-Path $RepoRoot 'scripts\build-libopus-ohos.sh')
if ($LASTEXITCODE -ne 0) {
  throw "OHOS libopus build failed with exit code $LASTEXITCODE."
}

$nativeLib = Join-Path $RepoRoot 'target\aarch64-unknown-linux-ohos\release\librustdesk_native_har.so'
$distLib = Join-Path $RepoRoot 'dist\arm64-v8a\librustdesk_native_har.so'
$buildStartedAt = Get-Date

Clear-NativeOutputs -IncludeTarget

Write-Host "OHOS_NDK_HOME=$env:OHOS_NDK_HOME"
Write-Host "OHOS target=$TargetTriple"
Write-Host "OHOS linker=$env:CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER"
Write-Host "OHOS vcpkg prefix=$env:OHOS_VCPKG_PREFIX"
Write-Host "CARGO_BUILD_JOBS=$env:CARGO_BUILD_JOBS"
Write-Host "MAKEFLAGS=$env:MAKEFLAGS"

Write-Host 'Building RustDesk native HAR for arm64-v8a...'
ohrs build --release -a aarch
if ($LASTEXITCODE -ne 0) {
  throw "ohrs build failed with exit code $LASTEXITCODE."
}

if (-not (Test-Path -LiteralPath $nativeLib)) {
  throw "ohrs build did not produce $nativeLib."
}

$nativeLibInfo = Get-Item -LiteralPath $nativeLib
if ($nativeLibInfo.LastWriteTime -lt $buildStartedAt.AddSeconds(-1)) {
  throw "ohrs build did not refresh $nativeLib. Refusing to package a stale native library."
}

if (-not (Test-Path -LiteralPath $distLib)) {
  throw "ohrs build did not produce $distLib."
}

$distLibInfo = Get-Item -LiteralPath $distLib
if ($distLibInfo.LastWriteTime -lt $buildStartedAt.AddSeconds(-1)) {
  throw "ohrs build did not refresh $distLib. Refusing to package a stale native library."
}

Write-Host 'Packaging HAR...'
Clear-NativeOutputs
ohrs artifact
if ($LASTEXITCODE -ne 0) {
  throw "ohrs artifact failed with exit code $LASTEXITCODE."
}

Write-Host "Done: $RepoRoot\package.har"
