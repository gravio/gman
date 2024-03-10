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
arch="x64"

# build the binary 
cargo build $mode

# generate sbom
cargo-sbom > $target_dir/sbom.json

# generate checksum of artifacts
checksums -v -c $target_dir --force

zip_name=${bin_name}_${os_type}_$arch.zip
zip $zip_name $target_dir/$bin_name $target_dir/sbom.json $target_dir/release.hash

# re-checksum for zip file
checksums -v -c $target_dir --force

echo ""
echo "Created release zip at $target_dir/$zip_name"