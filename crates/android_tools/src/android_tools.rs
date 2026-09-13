use anyhow::{Context as _, Result, bail, ensure};
pub mod kotlin;
use serde::Deserialize;
use std::{
    env, fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub serial: String,
    pub state: String,
    pub model: String,
}

impl Device {
    pub fn is_available(&self) -> bool {
        self.state == "device"
    }
}

pub fn parse_devices(output: &str) -> Result<Vec<Device>> {
    let mut lines = output
        .lines()
        .skip_while(|line| !line.starts_with("List of devices attached"));
    ensure!(lines.next().is_some(), "ADB did not return a device list");
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            // Wireless mDNS serials can contain spaces. ADB places the state after
            // the complete serial, so find the last state token before metadata.
            let (serial, state, details) = line
                .match_indices(char::is_whitespace)
                .rev()
                .find_map(|(offset, _)| {
                    let remainder = line.get(offset..)?.trim_start();
                    let state = if remainder.starts_with("no permissions") {
                        "no permissions"
                    } else {
                        remainder.split_whitespace().next()?
                    };
                    matches!(
                        state,
                        "device"
                            | "offline"
                            | "unauthorized"
                            | "authorizing"
                            | "connecting"
                            | "bootloader"
                            | "recovery"
                            | "rescue"
                            | "sideload"
                            | "host"
                            | "unknown"
                            | "no permissions"
                    )
                    .then_some((
                        line.get(..offset)?.trim_end(),
                        state,
                        remainder,
                    ))
                })
                .context("ADB device state is missing or unsupported")?;
            ensure!(!serial.is_empty(), "Device serial is missing");
            let model = details
                .split_whitespace()
                .find_map(|field| field.strip_prefix("model:"))
                .unwrap_or(serial)
                .replace('_', " ");
            Ok(Device {
                serial: serial.into(),
                state: state.into(),
                model,
            })
        })
        .collect()
}

pub fn adb_path() -> Result<PathBuf> {
    let executable = if cfg!(windows) { "adb.exe" } else { "adb" };
    for variable in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(root) = env::var_os(variable).filter(|value| !value.is_empty()) {
            let path = PathBuf::from(root).join("platform-tools").join(executable);
            ensure!(
                path.is_file(),
                "{variable} does not contain platform-tools/{executable}"
            );
            return Ok(path);
        }
    }
    if let Ok(path) = which::which(executable) {
        return Ok(path);
    }
    if let Some(home) = dirs::home_dir() {
        let root = if cfg!(target_os = "macos") {
            home.join("Library/Android/sdk")
        } else if cfg!(windows) {
            home.join("AppData/Local/Android/Sdk")
        } else {
            home.join("Android/Sdk")
        };
        let path = root.join("platform-tools").join(executable);
        if path.is_file() {
            return Ok(path);
        }
    }
    bail!("ADB was not found. Install Android SDK Platform Tools and set ANDROID_HOME.")
}

pub fn android_cli_path() -> Result<PathBuf> {
    which::which("android")
        .context("Android CLI was not found. Install Google's Android CLI and add it to PATH.")
}

pub fn is_gradle_project(root: &Path) -> bool {
    (root.join("gradlew").is_file() || root.join("gradlew.bat").is_file())
        && [
            "settings.gradle.kts",
            "settings.gradle",
            "build.gradle.kts",
            "build.gradle",
        ]
        .iter()
        .any(|name| root.join(name).is_file())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidTarget {
    pub module: String,
    pub variant: String,
    pub output_listing: PathBuf,
}

impl AndroidTarget {
    pub fn label(&self) -> String {
        format!("{} · {}", self.module, self.variant)
    }

    pub fn gradle_task(&self, prefix: &str, suffix: &str) -> String {
        let mut characters = self.variant.chars();
        let capitalized = characters
            .next()
            .map(|first| first.to_uppercase().collect::<String>() + characters.as_str())
            .unwrap_or_default();
        format!(
            "{}:{prefix}{capitalized}{suffix}",
            self.module.trim_end_matches(':')
        )
    }

    pub fn apk_paths(&self) -> Result<Vec<PathBuf>> {
        let listing = fs::read_to_string(&self.output_listing).with_context(|| {
            format!(
                "Build {} before running it: the APK listing is missing",
                self.label()
            )
        })?;
        let metadata_path = if listing.trim_start().starts_with('{') {
            self.output_listing.clone()
        } else {
            let relative = listing
                .lines()
                .find_map(|line| line.strip_prefix("listingFile="))
                .context("The APK redirect does not contain listingFile")?;
            self.output_listing
                .parent()
                .context("APK listing has no parent directory")?
                .join(relative)
        };
        let metadata: ApkMetadata = serde_json::from_str(&fs::read_to_string(&metadata_path)?)
            .context("Unable to read Android APK metadata")?;
        ensure!(
            metadata.version == 3,
            "Unsupported APK metadata version: {}",
            metadata.version
        );
        ensure!(
            metadata.artifact_type.kind == "APK",
            "This target does not produce APKs"
        );
        ensure!(
            metadata.variant_name == self.variant,
            "The APK metadata belongs to a different build variant; sync and build again"
        );
        ensure!(!metadata.elements.is_empty(), "The build produced no APKs");
        // ponytail: deploy one universal APK; add device-aware split selection before supporting filtered outputs.
        ensure!(
            metadata.elements.len() == 1
                && metadata
                    .elements
                    .iter()
                    .all(|element| element.filters.is_empty()),
            "Split or filtered APK outputs are not supported yet. Select a variant that produces one universal APK."
        );
        let directory = metadata_path
            .parent()
            .context("APK metadata has no parent directory")?
            .canonicalize()?;
        metadata
            .elements
            .into_iter()
            .map(|element| {
                let path = Path::new(&element.output_file);
                ensure!(
                    !path.as_os_str().is_empty()
                        && path
                            .components()
                            .all(|component| matches!(component, Component::Normal(_))),
                    "APK output must be relative to its metadata directory"
                );
                let path = directory
                    .join(path)
                    .canonicalize()
                    .context("The built APK is missing")?;
                ensure!(
                    path.starts_with(&directory)
                        && path.is_file()
                        && path.extension().is_some_and(|extension| extension == "apk"),
                    "APK output is not an APK inside its metadata directory"
                );
                ensure!(
                    !path.to_string_lossy().contains(','),
                    "Android CLI cannot accept an APK path containing a comma"
                );
                Ok(path)
            })
            .collect()
    }
}

// ponytail: Android CLI 1.0 exposes targets as text; replace this adapter when it offers a versioned JSON command.
pub fn parse_targets(output: &str) -> Result<Vec<AndroidTarget>> {
    let mut module = None;
    let mut variant = None;
    let mut targets = Vec::new();
    for line in output.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("Task: ") {
            ensure!(
                value.starts_with(':')
                    && value
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric()
                            || ":_-.$".contains(character)),
                "Android CLI returned an invalid Gradle module path"
            );
            module = Some(value.to_owned());
            variant = None;
        } else if let Some(value) = line.strip_prefix("Variant: ") {
            ensure!(
                !value.is_empty()
                    && value
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '_'),
                "Android CLI returned an invalid variant name"
            );
            variant = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("Output Listing File: ") {
            if value == "null" || value.is_empty() {
                continue;
            }
            let output_listing = PathBuf::from(value);
            ensure!(
                output_listing.is_absolute(),
                "Android CLI returned a relative APK listing path"
            );
            let target = AndroidTarget {
                module: module
                    .clone()
                    .context("Android CLI returned a target without a module")?,
                variant: variant
                    .clone()
                    .context("Android CLI returned a target without a variant")?,
                output_listing,
            };
            ensure!(
                !targets
                    .iter()
                    .any(|previous: &AndroidTarget| previous.module == target.module
                        && previous.variant == target.variant),
                "Android CLI returned duplicate build targets"
            );
            targets.push(target);
        }
    }
    ensure!(
        !targets.is_empty(),
        "No Android application variants were found. Open a Gradle project with an Android application module and sync again."
    );
    targets
        .sort_by(|left, right| (&left.module, &left.variant).cmp(&(&right.module, &right.variant)));
    Ok(targets)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApkMetadata {
    version: u32,
    artifact_type: ArtifactType,
    variant_name: String,
    elements: Vec<ApkElement>,
}

#[derive(Deserialize)]
struct ArtifactType {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApkElement {
    output_file: String,
    filters: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_targets_devices_and_artifacts() -> Result<()> {
        let devices = parse_devices(
            "* daemon started successfully *\nList of devices attached\nemulator-5554 device product:sdk model:Pixel_6 transport_id:1\nusb-123 unauthorized usb:1\nremote:5555 offline\n",
        )?;
        assert_eq!(devices.len(), 3);
        assert_eq!(
            devices.first().map(|device| device.model.as_str()),
            Some("Pixel 6")
        );
        assert_eq!(
            devices
                .iter()
                .filter(|device| device.is_available())
                .count(),
            1
        );
        assert!(parse_devices("List of devices attached\n")?.is_empty());
        assert!(parse_devices("adb: cannot connect").is_err());
        assert!(parse_devices("List of devices attached\ninvalid").is_err());
        let serial = "adb-test-device (2)._adb-tls-connect._tcp";
        let wireless = parse_devices(&format!(
            "List of devices attached\n{serial} device product:phone model:CPH2689 device:phone transport_id:11\n"
        ))?;
        assert_eq!(
            wireless,
            vec![Device {
                serial: serial.into(),
                state: "device".into(),
                model: "CPH2689".into(),
            }]
        );
        assert!(wireless.first().is_some_and(Device::is_available));
        let unavailable = parse_devices(
            "List of devices attached\nusb-123 no permissions (user is not in the plugdev group)\n",
        )?;
        assert_eq!(
            unavailable.first().map(|device| device.state.as_str()),
            Some("no permissions")
        );

        let directory = tempfile::tempdir()?;
        let root = directory.path().join("project with spaces $literal");
        fs::create_dir(&root)?;
        fs::write(root.join("gradlew"), "")?;
        fs::write(root.join("settings.gradle.kts"), "")?;
        assert!(is_gradle_project(&root));
        assert!(!is_gradle_project(directory.path()));
        let output_listing = root.join("redirect.txt");
        let description = format!(
            "Task: :mobile:application\n  Variants:\n    Variant: freeDebug\n      Output Listing File: {}\n",
            output_listing.display()
        );
        let targets = parse_targets(&description)?;
        let target = targets.first().context("Expected the described target")?;
        assert_eq!(
            target.gradle_task("assemble", ""),
            ":mobile:application:assembleFreeDebug"
        );
        assert_eq!(
            target.gradle_task("test", "UnitTest"),
            ":mobile:application:testFreeDebugUnitTest"
        );
        assert!(
            parse_targets("Task: :app\n Variant: debug\n Output Listing File: relative.json")
                .is_err()
        );
        assert!(parse_targets("Output Listing File: /tmp/metadata.json").is_err());
        assert!(parse_targets("No variants").is_err());
        assert!(parse_targets(&format!("{description}{description}")).is_err());
        assert!(target.apk_paths().is_err());

        fs::write(
            &output_listing,
            "#- File Locator -\nlistingFile=output-metadata.json\n",
        )?;
        fs::write(root.join("app.apk"), b"test fixture")?;
        let metadata_path = root.join("output-metadata.json");
        let metadata = serde_json::json!({"version": 3, "artifactType": {"type": "APK"}, "variantName": "freeDebug", "elements": [{"filters": [], "outputFile": "app.apk"}]});
        fs::write(&metadata_path, serde_json::to_vec(&metadata)?)?;
        assert_eq!(
            target.apk_paths()?,
            vec![root.join("app.apk").canonicalize()?]
        );
        for field in ["../app.apk", "/tmp/app.apk", "missing.apk"] {
            let mut invalid = metadata.clone();
            invalid["elements"][0]["outputFile"] = field.into();
            fs::write(&metadata_path, serde_json::to_vec(&invalid)?)?;
            assert!(target.apk_paths().is_err());
        }
        let mut invalid = metadata.clone();
        invalid["variantName"] = "release".into();
        fs::write(&metadata_path, serde_json::to_vec(&invalid)?)?;
        assert!(target.apk_paths().is_err());
        let mut invalid = metadata;
        invalid["elements"][0]["filters"] =
            serde_json::json!([{"filterType": "ABI", "value": "x86"}]);
        fs::write(&metadata_path, serde_json::to_vec(&invalid)?)?;
        assert!(target.apk_paths().is_err());
        Ok(())
    }
}
