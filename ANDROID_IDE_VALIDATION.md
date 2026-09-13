# Android IDE prototype: validation and handoff

Session: 13–14 September 2026. Platform: Apple Silicon macOS. All changes and stack
branches remain local; no branches were pushed and no GitHub PRs were created.

This is a working edit/build/run prototype with an Android Studio-inspired
shell. It is not ready to replace Android Studio for projects that require
reliable Kotlin refactoring, Android debugging, or Compose previews. The
[research and implementation plan](ANDROID_IDE_PLAN.md) covers the longer-term
work; [the review guide](ANDROID_IDE_REVIEW.md) describes the local stack.

## Run it

From this checkout, launch the already-built optimized app:

```sh
script/android-ide --release --skip-build examples/android-ide
```

The launcher creates a macOS development bundle under `target/android-ide` and
uses `target/android-ide/profile` for configuration and extensions. It preserves
existing settings. New profiles disable auto-update and telemetry, enable the
Kotlin extension, and select the community Kotlin server. This is a development
launcher, not a signed installer or an independently branded release.

1. Open the Android tool window using the hammer on the right tool rail.
2. Save any Gradle-file edits, then choose **Sync project**. Four runnable
   variants should appear for `:mobile`. Sync currently reads files from disk;
   Build/Run/Test/Lint use the task system's save-before-run behavior.
3. Select **:mobile · demoDebug** or **:mobile · fullDebug**.
   The choice is remembered for this project and restored after the next sync.
4. Choose **Configure Kotlin**. It builds that variant, exports the evaluated
   compile classpath, and configures the project with a JDK 21 language server.
   The Kotlin extension must be installed; new launcher profiles request it
   automatically. Repeat setup after changing variants or dependencies.
5. Select a connected device, or choose an existing AVD using **Start emulator…**.
6. Choose **Run**. The app displays the selected flavor, its application ID,
   and `Android library connected`.
7. Use **Test**, **Lint**, or **Open Logcat** as needed. Build output remains in
   ordinary task terminals. Ctrl+C interrupts the focused command or Logcat.
8. Use **Stop emulator** when finished. This preserves the AVD and releases its
   VM memory. It is disabled for a physical device.

The project is a real Gradle build and participates in Zed's existing workspace
trust flow. Sync and build execute project code only after the project is
trusted. Opening an arbitrary folder does not automatically build it or install
SDK packages.

## What is implemented and what was actually checked

| Area | Result and scope |
| --- | --- |
| Appearance | Studio Dark and Studio Light render in the native app. Bundled JetBrains Mono, compact typography, left Project pane, side tool rails, bottom output, and top Android controls are present. |
| Layout | Both themes checked at the normal window size; a smaller light-theme window retained an editor and scrollable Android controls. Dock geometry and panel movement have GPUI regression coverage. |
| Keybindings | Existing JetBrains map is the default. Native macOS Build, Run, Sync, and Logcat were exercised. Linux/Windows asset precedence is tested, but those operating systems were not run. |
| Project sync | Uses evaluated Android CLI output. Single-module reference and two-module smoke project both synchronize. Libraries are dependencies, not misleading runnable targets. |
| Variant handling | `:mobile` has demo/full flavors and debug/release build types. Both debug flavors build, test, and deploy with distinct application IDs. No `:app` assumption. |
| Variant restoration | The selected module/variant is stored in Zed's local database, separate from project files. A real quit/relaunch/Sync restored `:mobile · fullDebug`, and Run deployed that flavor without reselection. Fresh sync supplies current artifact paths. A removed variant remains unselected with a visible explanation. Tests cover project isolation and changed output paths. |
| Devices | Explicit serial selection; offline and unauthorized entries retained. Missing device state is visible. Physical-device stop is rejected in tests. |
| Emulator lifecycle | Native Start/Stop exercised on the existing `medium_phone` AVD. The optimized build also started `Pixel_6a` and deployed both flavors there. Stop refreshes the device list without deleting or recreating AVDs. |
| Build and Run | Save-before-build, selected variant, AGP output metadata, explicit device, and visible output. Build failure prevents deployment. Repeated run after fixing an intentional Kotlin type error showed the changed greeting on the emulator. |
| APK selection | Existing AGP metadata/redirects are read. The implementation accepts a single universal APK and rejects ambiguous, filtered/split, missing, traversing, or variant-mismatched artifacts. Split APK installation is deferred. |
| Test and Lint | Native task actions passed. Direct smoke-project builds/test tasks passed for both debug flavors; lint passed with warnings listed below. |
| Logcat | Real selected-device `adb logcat -v threadtime` in a terminal with bounded history. Ctrl+C stops the stream. No dedicated filter/table UI yet. |
| Kotlin Android APIs | Android/Compose resolution, useful hover/completion, and deliberate type-error diagnostics exercised on the reference project. |
| Generated symbols | Smoke `R`, `BuildConfig`, and a Kotlin call to the app's Java helper resolve after setup. Variant changes regenerate the selected `R.jar` and Java output classpath. |
| Library navigation | Native hover and Cmd+B navigate from `MainActivity.kt` to `LibraryGreeting.kt` in `:greeting`. The library's compiled classes JAR appears in the exported classpath. |
| Kotlin rename | **Failed.** Shift+F6 on the library object returned a server internal error with `KotlinFrontEndException`. No source files changed. Do not advertise reliable rename/refactoring. |
| Java editing | **Android import failed.** Java extension 6.8.26 / JDT LS 1.61.0 reported the probe outside the app classpath with defaults and with Android support enabled. Building Java and resolving its symbols from Kotlin work; Android-aware Java editing and mixed-language rename are not supported. |
| Cancellation | A temporary 60-second Gradle task was interrupted using Ctrl+C in the native terminal. Controls recovered, and the restored build passed in 2 seconds. Current task API displays interruption as command failure. |
| Configuration safety | Existing user hooks are preserved with an actionable error. Generated files use atomic replacement. Settings updates preserve unrelated settings/comments and reject stale concurrent settings edits. |
| Isolation | Development launcher profile, bundle identifiers, generated caches, telemetry/update defaults, existing-profile preservation, relative path handling, and running-app guard were checked. |
| Helper cleanup | After quitting the Java probe, a process check found no remaining IDE or language-server commands associated with the isolated profile. Shared Gradle/ADB services are separate; this is an app-quit check, not a project-close stress test. |

## Shortcut reference

| Operation | macOS | Linux/Windows mapping |
| --- | --- | --- |
| Find Action | Cmd+Shift+A | Ctrl+Shift+A |
| Build selected Android variant | Cmd+F9 | Ctrl+F9 |
| Run selected Android variant | Ctrl+R | Shift+F10 |
| Sync Android project | Cmd+Option+Y | Ctrl+Alt+Y |
| Open selected-device Logcat | Cmd+6 | Alt+6 |

Sync originally lost to the default editor's Git Stage shortcut. The regression
test first reproduced `git::ToggleStaged` instead of `android::SyncProject`, then
passed after the Android bindings were given full-editor context as well as
workspace context. Native editor-focus Sync was rechecked after that fix.

## Language-server research changed the implementation choice

The official Kotlin LSP advertises experimental AGP support and remains Alpha;
KMP support is still under development. These are vendor-supported scope
statements, not evidence that this fixture works with that server.
[Kotlin documentation](https://kotlinlang.org/docs/kotlin-lsp.html).

The installed Zed Kotlin extension was version 0.3.2. Its default official
server download, `262.9593.0`, exited because that build had expired when tested
on 13 September. No expiry check was bypassed. JetBrains separately documents
30-day preview-build expiry for its Java/Kotlin VS Code integration and an
Ultimate subscription requirement after preview; do not assume this is a free,
unrestricted redistribution path for this fork.
[JetBrains installation and preview terms](https://www.jetbrains.com/help/intellij-vscode/get_started_vs_code.html).

The working compatibility path uses `fwcd/kotlin-language-server` 1.3.13 with
JDK 21 and its documented project classpath hook. JBR 25 produced a Java-version
parsing error, so Gradle and language-server Java runtimes are selected
separately. The community repository now describes itself as deprecated in
favor of the official server. Its successful diagnostics/navigation here do not
make it a sustainable production language engine.
[Community server status and classpath hooks](https://github.com/fwcd/kotlin-language-server).

The generated Gradle init task reads the selected Kotlin task's libraries and
the corresponding Java compiler output. The latter is necessary for generated
`BuildConfig` and Java helpers. It works when the Java task is `NO-SOURCE` and
with Gradle configuration-cache reuse. It does not parse Gradle source code or
change the user's Gradle build files.

Known ceiling: one selected variant supplies a workspace-wide classpath. This
does not model different variants in different modules, test-specific
classpaths, KMP, included builds, or source-set visibility with Android Studio's
precision. The next language milestone should evaluate a legally distributable,
maintained server against the same failing rename and project-model fixtures.

### Java import spike

Installed the official Zed Java extension **6.8.26**, which downloaded Eclipse
JDT LS **1.61.0**. A disposable Java file in the reference app accessed
`android.os.Build.VERSION.SDK_INT` and generated `R.string.app_name`. The server
imported the Gradle project, but reported that the file was not on the app
classpath and restricted diagnostics to syntax. Its log contains Java Model
error 969 for the existing source directory.

The second attempt explicitly enabled
`lsp.jdtls.initialization_options.settings.java.jdt.ls.androidSupport.enabled`
and automatic build-configuration updates. The test-owned import cache was
moved aside, then the server restarted and reproduced the same failure. The
temporary probe and settings were restored afterward. This is a concrete
compatibility failure on AGP 9.4.0 / Gradle 9.6.1, not a claim that JDT LS fails
for every Android project. The extension remains installed in the isolated
development profile, but is not enabled automatically in new launcher profiles.

Sources for the tested configuration:
[Zed Java extension configuration](https://github.com/zed-extensions/java),
[JDT LS experimental Android support](https://github.com/eclipse-jdtls/eclipse.jdt.ls).
The logs and exact probe are retained under `target/android-ide/validation`.

## Reproducible build environment

| Component | Verified version/location |
| --- | --- |
| Host | Apple Silicon, 16 GiB RAM, macOS 26.6.2 |
| Rust | Repository-pinned 1.98.1, installed with rustfmt, Clippy, rust-src and rust-analyzer |
| Xcode | 26.6, build 17F113; Metal compiler available |
| Android Studio reference | Quail 4, 2026.1.4, `/Applications/Android Studio.app` |
| Gradle JVM used by launcher | Android Studio JBR 25.0.3 when `JAVA_HOME` is unset |
| Kotlin JVM | Temurin 21.0.12.1 in `/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home` |
| Android CLI | 1.0.16261425, `/opt/homebrew/bin/android` |
| SDK | `/Users/anil/Library/Android/sdk`; platform 37 used for fixture |
| Smoke build | AGP 9.4.0, Gradle 9.6.1, Kotlin Compose plugin 2.2.10, Compose BOM 2026.02.01 |
| Tools installed for this session | Pinned Rust components and the official `github/gh-stack` extension |
| Tools reused | Xcode/Metal, CMake, Ninja, Homebrew, Android Studio/SDK/CLI, JDK 21, ADB/emulator, telegram-send |

The upstream LiveKit build helper could not retrieve its WebRTC archive through
its normal download path. The matching official archive was downloaded directly
and its extracted headers/libraries were supplied through the helper's supported
`LK_CUSTOM_WEBRTC` override. A copy is retained at the following task-local path:

```sh
export LK_CUSTOM_WEBRTC="$PWD/target/android-ide/dependencies/webrtc-0001d84-4/mac-arm64-release"
cargo build --release -p zed --bin zed --locked -j 4
```

If that ignored cache is removed, retrieve the matching
[official WebRTC archive](https://github.com/zed-industries/livekit-rust-sdks/releases/download/webrtc-0001d84-4/webrtc-mac-arm64-release.zip)
again or let upstream's download work. Do not reuse these prebuilt libraries
after a LiveKit/WebRTC ABI update without checking the dependency's tag.
The launcher also honors `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS`.

The Gradle wrapper was regenerated using Gradle 9.6.1 and its JAR hash was
matched to the official checksum:

```text
wrapper JAR SHA-256:
497c8c2a7e5031f6aa847f88104aa80a93532ec32ee17bdb8d1d2f67a194a9c7
distribution SHA-256 (pinned in gradle-wrapper.properties):
9c0f7faeeb306cb14e4279a3e084ca6b596894089a0638e68a07c945a32c9e14
```

Sources: [wrapper checksum](https://downloads.gradle.org/distributions/gradle-9.6.1-wrapper.jar.sha256),
[distribution checksum](https://downloads.gradle.org/distributions/gradle-9.6.1-bin.zip.sha256).

## Checks performed

The following checks passed on the combined source changes, including the
shortcut-context and emulator-stop fixes:

```sh
cargo test -p android_tools -j 6
cargo test -p android_ui -j 6
cargo test -p workspace test_toggle_docks_and_panels -j 6
./script/clippy -p android_tools
./script/clippy -p android_ui
./script/clippy -p workspace -p project_panel -p ui --features gpui/inspector
bash -n script/android-ide
git diff --check
git -c core.whitespace=blank-at-eol,blank-at-eof,space-before-tab,cr-at-eol \
  diff --check main..HEAD -- . ':(exclude)assets/fonts/jetbrains-mono/OFL.txt'
```

The complete-stack whitespace check accepts the generated Windows wrapper's
CRLF endings and excludes the unmodified vendored font license, which has one
upstream trailing space. An unconfigured `git diff --check main..HEAD` reports
those vendor-file differences; the source/doc check above passes.

Set `LK_CUSTOM_WEBRTC` as above for commands that build the relevant native
dependencies. The core crate has two focused tests covering parsers/artifact
selection and the real filesystem/classpath hook. The Android GPUI test covers
trust, multiple roots, device state, action dispatch, dock movement, settings
preservation, persisted variant selection, and keymap precedence. The existing workspace dock regression was
extended to catch an editor that accidentally collapses after adding tool rails.

The shell Clippy command explicitly enables `gpui/inspector`: without it, the
upstream all-features UI build fails on its inspector derive feature wiring.
No unrelated upstream source fix was added. Release linking also emits an
upstream large-`__eh_frame` warning; Cargo reports future incompatibility in
`block` 0.1.6. These were warnings, not failed validation.

Smoke-project verification:

```sh
cd examples/android-ide
export JAVA_HOME='/Applications/Android Studio.app/Contents/jbr/Contents/Home'
export ANDROID_HOME="$HOME/Library/Android/sdk"
./gradlew :mobile:assembleDemoDebug :mobile:assembleFullDebug \
  :mobile:testDemoDebugUnitTest :mobile:testFullDebugUnitTest \
  :mobile:lintDemoDebug
```

With the Android library module added, this passed in 9 seconds: 135 actionable
tasks, 72 executed and 63 up to date. These are warm local timings, not a build
speed comparison. Lint had five warnings: newer Gradle, Compose BOM and Activity
versions exist; the smoke manifest has a backup-policy warning and no launcher
icon. No lint errors. Versions were kept consistent with the verified fixture
rather than expanding this test into dependency upgrades.

## Optimized build and initial performance measurements

The final native source, including variant restoration, compiled successfully
in **18 minutes 26 seconds** with four jobs. Its optimized executable reports
`1651d598ccc367f3b1a6dbbc46ef5612a9c59103`. Combined tests and Clippy also passed
after the local stack rebase. The report commit changes documentation only.
The final executable passed native restart/Sync variant restoration, emulator
startup, and Ctrl+R build/deploy of `:mobile · fullDebug` on `medium_phone`.
ADB confirmed `dev.zed.androidsample.full` as the resumed activity. The IDE's
Stop emulator action then completed and ADB listed no connected devices.

The optimized IDE is left open on the smoke project with `fullDebug` selected.
Android Studio and the test emulators are closed; the AVDs remain available for
the next run. Gradle and ADB may retain their normal shared daemon processes.

The optimized build before the variant-restoration fix passed in 18 minutes
53 seconds using four Cargo jobs. Its executable reports
`93d7661d5f36e208286d10dfd4be459332441142`.
The later smoke-fixture update added an Android library without changing the
native source. Both flavors deployed successfully on `Pixel_6a` from this build.
The original optimized baseline build also passed,
and repeated debug builds were used for the shorter test/fix cycles.

### Warm startup observation

Three launches of the optimized `93d7661d5f` executable against the same already-used
profile and smoke project produced **0.367, 0.169, and 0.165 seconds** from process
creation to receiving Zed's first `Rendered first frame` log. Median: **0.169
seconds**. A monotonic Python timer read the child process's PTY; no UI-automation
tool latency is included. The actual sample-project window was checked after
each launch. No Rust compilation was running during these trials.

This log is emitted at the first workspace render call. It does not establish
frame presentation, editor input latency, project sync completion, or language
server readiness. These are warm-cache observations, with no Android Studio
startup comparison. Raw `startup-1.json` through `startup-3.json`, corresponding
logs, and the measurement script are retained in the local validation directory.

### Idle memory observation

Three samples were taken from 22:27:52 through 22:28:32 IST with the original
small Android Studio-generated Compose project open in both IDEs. Android Studio
and the optimized fork were idle; no Rust compiler was running during sampling.
The measurements used `vmmap` physical footprint because earlier compilation
had caused memory compression, making RSS alone misleading.

| Process or process group | Median physical footprint | Range |
| --- | ---: | ---: |
| Optimized Android IDE editor | 297.8 MiB | 297.7–297.8 MiB |
| Its Kotlin language server | approximately 1.30 GiB | rounded by vmmap |
| Editor plus Kotlin server | approximately 1.59 GiB | approximately 1.59 GiB |
| Android Studio | approximately 3.10 GiB | rounded by vmmap |
| Shared Gradle daemon (JBR 25) | approximately 1.80 GiB | rounded by vmmap |
| Shared Kotlin compiler daemon | 553.7 MiB | 553.7 MiB |
| Additional Gradle toolchain daemon | 701.6 MiB | 701.6 MiB |
| Shared emulator | approximately 5.30 GiB | rounded by vmmap |

The editor-plus-language-server group was about 49% smaller in these samples.
This is an initial observation on one small project, not a feature-equivalent
benchmark or a product-wide memory claim. Server functionality, JVM versions,
process age, compression, and warm caches differ. Gradle and emulator costs are
substantial and do not disappear when changing editors. Android Studio and the
fork may also cause distinct compatible/incompatible Gradle daemons to coexist.

The measured optimized executable reported revision `c0bf3f23`, an earlier
implementation build with the shell and Kotlin setup. Subsequent fixes add
generated Java outputs, emulator controls, and shortcut precedence. Do not label
this sample set as a measurement of the final source revision.

Raw JSON and all three per-process `vmmap` reports are retained in
`target/android-ide/validation/memory-baseline.json` and
`target/android-ide/validation/baseline-*.txt`. This ignored directory is local
evidence, not a committed binary payload.

## Remaining acceptance gates

1. **Language engine and project model:** resolve the observed rename failure;
   choose a maintained server with suitable terms; verify Java editing,
   per-module source sets, tests, KMP, included builds, and stale-model recovery.
2. **Android debugger:** prove deploy/attach, Kotlin breakpoints, stepping,
   evaluation, detach, and process death through a compatible adapter before
   exposing a working Android Debug button.
3. **Compose/XML preview:** demonstrate standalone rendering and resource/variant
   correctness with a bounded worker process. This prototype does not render
   previews; `@Preview` remains useful when opened in Android Studio.
4. **Project view and device tools:** logical Android grouping, native Logcat
   filters, AVD/SDK creation and management, split APK support, and an embedded
   emulator need their own tested increments.
5. **Performance:** repeat measurements on representative large projects, record
   input/frame latency and LSP-ready time, test repeated sync/close cycles, and
   include total helper-process cost. Deep file scanning is enabled so nested
   Kotlin files appear in projects without Git; its cost on large or overly
   broad roots still needs measurement. No responsiveness comparison is
   inferred from editor memory alone.
6. **Distribution:** validate Linux and Windows; separate product identity,
   logs/caches/update/crash policy from upstream; review license obligations;
   provide a signed installer and upgrade path. The current launcher is macOS
   development tooling only.

## Local evidence

The ignored `target/android-ide/validation` directory preserves the actual
screenshots and logs without adding build artifacts to the review stack:

| Artifact | What it demonstrates |
| --- | --- |
| `zed-android-studio-reference.png` | Installed Android Studio layout. This early capture predates recovery of the reference project's initial sync failure. |
| `zed-android-studio-shell.png` | Running native fork with Studio-style tool rails and build/run output. |
| `zed-final-native-run.png` | Final optimized executable after successful full-flavor deployment on `medium_phone`. |
| `saved-variant-restored.png` | `fullDebug` restored by Sync after a real app quit and relaunch. |
| `zed-android-studio-light.png` | Real light-theme editor, project tree, Android controls, and successful build. |
| `zed-android-narrow.png` | Smaller window with Android controls still reachable by scrolling. |
| `smoke-full-final.png` | The full flavor on Pixel_6a, including its application ID and Android library output. |
| `zed-android-cancel.png` | Deliberately interrupted Gradle task and the current interruption-as-failure UI. |
| `zed-android-rename-error.png`, `zed-kotlin-rename-server-error.png` | Reproduced community Kotlin rename failure. |
| `zed-java-android-import-error.png`, `jdtls-android-import.log`, `AndroidIdeJavaProbe.java` | Failed Android Java import after enabling JDT LS Android support. |
| `zed-final-combined-tests.log`, `zed-final-combined-clippy.log` | Combined stack checks after the variant-restoration change. |
| `zed-selection-release-build.log`, `zed-final-system-specs.txt` | Successful final optimized build and its source revision. |
| `memory-baseline.json`, `baseline-*.txt` | Three raw per-process memory samples. |
| `helper-cleanup.json` | Scoped process check after closing the development app and Java probe. |
| `startup-*.json`, `startup-*.log`, `zed-startup-measure.py` | Monotonic warm first-workspace-render measurements and harness. |

All requested progress messages were sent using `telegram-send` to the
explicitly approved configured destination. Screenshots show running local
software, not generated mockups. The root README review notice remains in place
for the human author to remove only after reviewing the stack.
