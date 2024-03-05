$mode = "--release"
$target_dir = ".\target\release"
$bin_name = "graviomanager"

# build the binary 
cargo build $mode

# generate sbom
cargo sbom > $target_dir\sbom.spdx

# generate checksum of artifacts
checksums -v -c $target_dir --force


# zip the file for packaing
$compress = @{
    Path             = "$target_dir\${bin_name}.exe", "$target_dir\sbom.spdx", "$target_dir\release.hash"
    CompressionLevel = "Optimal"
    DestinationPath  = "$target_dir\${bin_name}_win_x64.zip"
}
Compress-Archive @compress -Force

# zip file re-checksum
checksums -v -c $target_dir --force