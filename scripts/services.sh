# rm data/arbitration/*.snark
# rm data/arbitration/*.yul
# rm data/arbitration/*.calldata
# rm data/arbitration/*.pk
# rm cache_data/arbitration/*.snark
# rm cache_data/arbitration/*.pk

rm services
cargo clean
cargo build --release --bin services
cp target/release/services .
./services