mode="--release"
target_dir="./target/release"
bin_name="graviomanager"

os_type=""
if [[ "$OSTYPE" == "linux"* ]]; then
        os_type="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
        os_type="mac"
elif [[ "$OSTYPE" == "cygwin" ]]; then
         os_type="windows"
elif [[ "$OSTYPE" == "msys" ]]; then
         os_type="windows"
elif [[ "$OSTYPE" == "win32" ]]; then
        os_type="windows"
elif [[ "$OSTYPE" == "freebsd"* ]]; then
        os_type="bsd"
else
        os_type="unknown"
fi

# build the binary 
cargo build $mode

# generate sbom
cargo-sbom > $target_dir/sbom.spdx

# generate checksum of artifacts
checksums -v -c $target_dir --force

zip "$bin_name"_x64_$os_type.zip $target_dir/$bin_name $target_dir/sbom.spdx $target_dir/release.hash

# re-checksum for zip file
checksums -v -c $target_dir --force