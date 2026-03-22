#!/bin/bash
# Build test WASM fixtures for integration tests.
# Requires: rustup target add wasm32-wasip1

set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
FIXTURES="$DIR/fixtures"
SRCDIR="$DIR/fixture_src"

mkdir -p "$FIXTURES" "$SRCDIR"

# hello.wasm — prints to stdout
cat > "$SRCDIR/hello.rs" << 'EOF'
fn main() {
    println!("hello from wytti");
}
EOF

# infinite.wasm — loops forever (for timeout tests)
cat > "$SRCDIR/infinite.rs" << 'EOF'
fn main() {
    loop {
        std::hint::black_box(42);
    }
}
EOF

# args.wasm — prints args
cat > "$SRCDIR/args.rs" << 'EOF'
fn main() {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        println!("{arg}");
    }
}
EOF

# exit_code.wasm — exits with code 1
cat > "$SRCDIR/exit_code.rs" << 'EOF'
fn main() {
    std::process::exit(1);
}
EOF

for src in hello infinite args exit_code; do
    rustc --target wasm32-wasip1 "$SRCDIR/$src.rs" -o "$FIXTURES/$src.wasm" 2>&1
    echo "built $src.wasm"
done

echo "all fixtures built"
