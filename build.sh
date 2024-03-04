mode="--release"
target_dir="./target/release"

# build the binary 
cargo build $mode

# generate sbom
cargo sbom > $target_dir\sbom.spdx

# generate checksum of artifacts
checksums -v -c $target_dir --force
