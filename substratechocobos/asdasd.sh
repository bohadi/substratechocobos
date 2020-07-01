./scripts/build.sh
cargo build --release
./target/release/substratechocobos purge-chain --dev
./target/release/substratechocobos --dev

