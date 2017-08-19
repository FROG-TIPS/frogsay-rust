$ErrorActionPreference = 'Stop';

$packageName= $env:ChocolateyPackageName
$toolsDir   = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$url        = 'https://github.com/chrlie/frogsay-rust/releases/download/3.0.0/frogsay-3.0.0-i686-pc-windows-gnu.zip'
$url64      = 'https://github.com/chrlie/frogsay-rust/releases/download/3.0.0/frogsay-3.0.0-x86_64-pc-windows-gnu.zip'

$packageArgs = @{
  packageName   = $packageName
  unzipLocation = $toolsDir
  fileType      = 'EXE'
  url           = $url
  url64bit      = $url64

  softwareName  = 'frogsay*'

  checksum      = '3657AC99399BB5BCB093BCDBF7BC513C69EF289DC55BE491895A1EFAD9394046'
  checksumType  = 'sha256'
  checksum64    = 'C02D38E438502554B711890CF7433A68D3054EDA92960A7E62C1D06A1A99297E'
  checksumType64= 'sha256'
}

Install-ChocolateyZipPackage @packageArgs
