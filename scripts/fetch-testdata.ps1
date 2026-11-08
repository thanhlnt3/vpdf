# Bộ tài liệu tham chiếu (task 1.5): tải từ $env:VPDF_TESTDATA_URL nếu có, nếu không thì tạo tại chỗ;
# sau đó kiểm tra SHA-256 theo testdata\MANIFEST.json.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo run -p vpdf-testdata --release -- fetch @args
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
