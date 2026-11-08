# Tải PDFium và qpdf dựng sẵn cho Windows x64 vào third_party\, kiểm tra SHA-256 (task 1.3, 1.4).
# Dùng: powershell -ExecutionPolicy Bypass -File scripts\fetch-native.ps1
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

$items = @(
  @{ Url = 'https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/8138/pdfium-win-x64.tgz';
     Sha = '8d48973e3eb472dda0f5f2435e595ac043f9f2219dd71883392f549445afa8a5'; Dest = 'third_party\pdfium\win-x64'; Kind = 'tgz' },
  @{ Url = 'https://github.com/qpdf/qpdf/releases/download/v12.4.2/qpdf-12.4.2-msvc64.zip';
     Sha = 'db87077e683630c1217e0e8f9a20a9749d952ab676e881c3689187763a5de25d'; Dest = 'third_party\qpdf\win-x64'; Kind = 'zip' }
)

foreach ($i in $items) {
  $file = Join-Path $env:TEMP ([IO.Path]::GetFileName($i.Url))
  Invoke-WebRequest -Uri $i.Url -OutFile $file -UseBasicParsing
  $got = (Get-FileHash -Algorithm SHA256 $file).Hash.ToLower()
  if ($got -ne $i.Sha) { throw "SHA-256 không khớp cho $($i.Url): $got" }
  New-Item -ItemType Directory -Force $i.Dest | Out-Null
  if ($i.Kind -eq 'tgz') { tar -xzf $file -C $i.Dest } else { Expand-Archive -Force $file $i.Dest }
}
Write-Host 'Đã tải thư viện native cho win-x64.'
