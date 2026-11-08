#!/usr/bin/env bash
# Bộ tài liệu tham chiếu (task 1.5): tải từ $VPDF_TESTDATA_URL nếu có, nếu không thì tạo tại chỗ;
# sau đó kiểm tra SHA-256 theo testdata/MANIFEST.json.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -p vpdf-testdata --release -- fetch "$@"
