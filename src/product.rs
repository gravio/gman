use std::{collections::HashMap, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{gman_error::GManError, platform::Platform};

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Product {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Flavors")]
    pub flavors: Vec<Flavor>,
}

#[derive(Serialize, Debug, PartialEq, Eq, Clone)]
pub enum PackageType {
    /// Windows UWP style,
    AppX,
    /// Traditional Windows installer
    Msi,
    /// Modern Windows MSI
    MsiX,
    /// Just a direct windows executable file
    StandaloneExe,
    /// Mac installation (image)
    App,
    /// Mac installation (zip)
    Pkg,
    /// Linux Debian package
    Deb,
    /// Android package
    Apk,
    /// iOS app package
    Ipa,
}

impl PackageType {
    pub fn supported_for_platform(&self, platform: &Platform) -> bool {
        match platform {
            Platform::Android => self == &PackageType::Apk,
            Platform::IOS => self == &PackageType::Ipa,
            Platform::Windows => {
                self == &PackageType::Msi
                    || self == &PackageType::MsiX
                    || self == &PackageType::AppX
            }
            Platform::Mac => self == &PackageType::Apk,
            Platform::RaspberryPi => self == &PackageType::Deb,
            Platform::Linux => self == &PackageType::Deb,
        }
    }
}
impl<'de> Deserialize<'de> for PackageType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;

        match value {
            serde_json::Value::String(val) => {
                let result = PackageType::from_str(&val).map_err(|_| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(&val),
                        &"one of {appx, msi, msix, app, pkg, deb, apk, ipa, standaloneexe}",
                    )
                })?;
                Ok(result)
            }
            _ => Err(serde::de::Error::custom(
                "Expected string for 'PackageType'",
            )),
        }
    }
}

impl FromStr for PackageType {
    type Err = GManError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "appx" => Ok(Self::AppX),
            "msi" => Ok(Self::Msi),
            "msix" => Ok(Self::MsiX),
            "standaloneexe" => Ok(Self::StandaloneExe),
            "app" => Ok(Self::App),
            "pkg" => Ok(Self::Pkg),
            "deb" => Ok(Self::Deb),
            "apk" => Ok(Self::Apk),
            "ioa" => Ok(Self::Ipa),
            _ => Err(GManError::new("Not a valid PackageType string")),
        }
    }
}
#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct TeamCityMetadata {
    #[serde(rename = "TeamCityId")]
    pub teamcity_id: String,
    #[serde(rename = "TeamCityBinaryPath")]
    pub teamcity_binary_path: std::path::PathBuf,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Flavor {
    #[serde(rename = "Platform")]
    pub platform: Platform,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "TeamCityMetadata")]
    pub teamcity_metadata: TeamCityMetadata,
    #[serde(rename = "PackageType")]
    pub package_type: PackageType,
    #[serde(rename = "Metadata")]
    pub metadata: Option<FlavorMetadata>,
    #[serde(rename = "Autorun", default = "default_bool::<false>")]
    pub autorun: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FlavorMetadata {
    /// for Windows AppX
    #[serde(rename = "NameRegex", skip_serializing_if = "Option::is_none")]
    pub name_regex: Option<String>,
    /// For Windows MSI
    #[serde(rename = "DisplayNameRegex", skip_serializing_if = "Option::is_none")]
    pub display_name_regex: Option<String>,

    /// For StandaloneEXE
    #[serde(rename = "InstallPath", skip_serializing_if = "Option::is_none")]
    pub install_path: Option<String>,

    /// For Mac App
    #[serde(rename = "CFBundleIdentifier", skip_serializing_if = "Option::is_none")]
    pub cf_bundle_id: Option<String>,
    /// For MacApp
    #[serde(rename = "CFBundleName", skip_serializing_if = "Option::is_none")]
    pub cf_bundle_name: Option<String>,
}

const fn default_bool<const V: bool>() -> bool {
    V
}

impl Flavor {
    pub fn empty() -> Self {
        Self {
            platform: Platform::platform_for_current_platform().unwrap(),
            id: "--".into(),
            package_type: PackageType::Msi,
            teamcity_metadata: TeamCityMetadata {
                teamcity_id: "--".into(),
                teamcity_binary_path: PathBuf::new(),
            },
            metadata: None,
            autorun: false,
        }
    }
}

impl Product {
    pub fn from_name<'a>(product_name: &'_ str, products: &'a Vec<Product>) -> Option<&'a Self> {
        products
            .iter()
            .find(|x| x.name.to_lowercase() == product_name.to_lowercase())
    }
}

#[cfg(test)]
mod tests {

    #[cfg(target_os = "macos")]
    #[test]
    fn test_parse_plist() {
        use plist::Value;
        use std::collections::HashMap;

        let plist_str = r#"
        <?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>BuildMachineOSBuild</key>
	<string>23C71</string>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>Gravio HubKit</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
	<key>CFBundleIconName</key>
	<string>AppIcon</string>
	<key>CFBundleIdentifier</key>
	<string>com.asteria.mac.gravio4</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>Gravio HubKit</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>5.2.1</string>
	<key>CFBundleSupportedPlatforms</key>
	<array>
		<string>MacOSX</string>
	</array>
	<key>CFBundleVersion</key>
	<string>8213</string>
	<key>DTCompiler</key>
	<string>com.apple.compilers.llvm.clang.1_0</string>
	<key>DTPlatformBuild</key>
	<string></string>
	<key>DTPlatformName</key>
	<string>macosx</string>
	<key>DTPlatformVersion</key>
	<string>14.2</string>
	<key>DTSDKBuild</key>
	<string>23C53</string>
	<key>DTSDKName</key>
	<string>macosx14.2</string>
	<key>DTXcode</key>
	<string>1520</string>
	<key>DTXcodeBuild</key>
	<string>15C500b</string>
	<key>LSMinimumSystemVersion</key>
	<string>10.15</string>
	<key>LSUIElement</key>
	<true/>
	<key>NSHumanReadableCopyright</key>
	<string>Copyright © 2018-2024 ASTERIA Corporation. All rights reserved.</string>
	<key>NSMainStoryboardFile</key>
	<string>Main</string>
	<key>NSPrincipalClass</key>
	<string>NSApplication</string>
	<key>SMPrivilegedExecutables</key>
	<dict>
		<key>com.asteria.mac.gravio.helper</key>
		<string>anchor apple generic and identifier "com.asteria.mac.gravio.helper" and (certificate leaf[field.1.2.840.113635.100.6.1.9] /* exists */ or certificate 1[field.1.2.840.113635.100.6.2.6] /* exists */ and certificate leaf[field.1.2.840.113635.100.6.1.13] /* exists */ and certificate leaf[subject.OU] = "3N2WH5W3MU")</string>
	</dict>
	<key>SUEnableAutomaticChecks</key>
	<true/>
	<key>SUFeedURL</key>
	<string>https://download.gravio.com/updatev5/macos/appcast.xml</string>
	<key>SUPublicDSAKeyFile</key>
	<string>dsa_pub.pem</string>
	<key>SUPublicEDKey</key>
	<string>hv+cM5PwRW8l+qA76FSNMi7CMSTzrqX/2OSIjV1hJRo=</string>
</dict>
</plist>
        "#;
        let pl: HashMap<String, Value> = plist::from_bytes(plist_str.as_bytes()).unwrap();

        println!("{:#?}", pl);
    }
}
