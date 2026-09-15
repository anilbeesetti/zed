# Android IDE smoke project

This small Compose app tests a module named `mobile`, two product flavors,
generated `R` and `BuildConfig` symbols, a Kotlin call into Java, and an Android
library dependency named `greeting`.

From the repository root, run:

```sh
script/android-ide --release examples/android-ide
```

Trusted Android projects sync automatically on open. Select **:mobile · demoDebug**
in the topbar and use **Configure community Kotlin** in the Android tools panel. Install the Kotlin extension from Extensions if it is
not already installed. Select a connected device or a stopped emulator in the topbar,
then **Run**; a stopped emulator is booted before deployment. The app should display `dev.zed.androidsample.demo`. Repeat with
`fullDebug`; it should display `dev.zed.androidsample.full`. Both variants should
also display `Android library connected`. Configure the selected Kotlin backend
again after switching variants so generated symbols use that variant.
Use **Stop emulator** when finished to release the VM's memory.

The community backend remains the default. To try the pinned official backend:

```sh
script/install-android-kotlin --backend official
script/android-ide --release --kotlin-backend official examples/android-ide
```

Use **Configure official Kotlin** in the Android tools panel. It generates the
selected variant's resources without assembling the app, so Kotlin compilation
errors do not block import. Resource-generation failures show degraded generated
symbol support while keeping the backend configured.

The Run menu's Android unit tests and lint actions operate on the selected
variant. To check the fixture directly:

```sh
cd examples/android-ide
./gradlew :mobile:assembleDemoDebug :mobile:testDemoDebugUnitTest :mobile:lintDemoDebug
```

The fixture requires Android SDK 37 and a JDK supported by Gradle 9.6.1. The
macOS launcher discovers the standard SDK and Android Studio runtime; command
line builds need `ANDROID_HOME` and `JAVA_HOME` set. Kotlin setup uses JDK 21
separately. Generated caches and machine-specific settings are ignored.

## Official Kotlin LSP protocol check

`script/test-kotlin-lsp --self-test` checks UTF-16 incremental edits, framing,
no-op document versions, readiness, and error classification without a server.
To run the real compatibility check, use an isolated sample copy and the pinned
official `263.4702.0` server:

```sh
fixture="$(mktemp -d)"
mkdir "$fixture/project"
tar --exclude=build --exclude=.gradle --exclude=.kotlin --exclude=.zed \
  --exclude=local.properties --exclude=workspace.json \
  -C examples/android-ide -cf - . | tar -C "$fixture/project" -xf -
export JAVA_HOME=/path/to/jdk-21
export ANDROID_HOME=/path/to/android-sdk
script/test-kotlin-lsp \
  --project "$fixture/project" \
  --server /path/to/kotlin-server-263.4702.0/bin/intellij-server \
  --output "$fixture/report" \
  --variant demoDebug
```

The project JDK is separate from the server's bundled Java runtime. The probe
uses the native Gradle importer and waits for both import success and actual
indexing completion. It immediately requests completion after each unsaved
incremental change, without sleeps or retries. It executes the returned completion
command, applies edits to its in-memory buffer, and checks the import, call and
caret response. It never saves its source edits or assembles the application.
Generated-resource checks can fail when the fixture has not generated resources.
For the default variant, generate just resources with
`"$fixture/project/gradlew" -p "$fixture/project" :mobile:processDemoDebugResources`,
then repeat the probe.

`results.json` records the negotiated capabilities, actual imported app/library
variants, source-attachment presence, document versions, request times, target
URIs/ranges, virtual content, and completion edits. `wire.jsonl` preserves both
protocol directions; import and indexing events and server errors are retained.
The output directory also holds server indexes for repeat-run comparisons.
Use a fresh output directory for cold-index measurements. Gradle can write to the
project and its normal caches; the probe briefly exports and removes
`workspace.json`, and refuses a fixture that already has that file.

Required assertion failures exit nonzero. The known original Compose source,
nested source navigation, and trailing-lambda gaps are reported separately;
add `--require-original-sources` to make original/nested-source checks required.
`--variant fullRelease` also checks that the dependent library selects `release`;
the tested server currently falls back to `greeting.debug`, which fails this gate.
This is a headless protocol check. Separate Zed client regressions cover
incremental completion acceptance, command edits, caret undo/redo, workspace-edit
failure reporting, and virtual-document ownership/lifecycle in
`crates/editor/src/editor_tests.rs` and `crates/project/tests/integration/lsp_store.rs`.
Expired completion sessions, real server semantics, and visible navigation latency
remain separate validation gates.

Run `script/install-android-kotlin`, `script/install-android-debugger`, and
`script/install-android-preview` once before launching the IDE. **Configure Java**
imports the selected variant into JDT LS; repeat Java and Kotlin setup after
changing variants or dependencies.

**Debug** builds and launches the selected app, then attaches the native debugger.
Set breakpoints on the return in `Greeting.java` and `LibraryGreeting.kt`; inspect
variables, step, and disconnect using the debugger controls. On macOS, Control-D
starts Android debugging, Command-Option-R continues, Shift-F8 steps out, and
Command-F2 disconnects. `script/test-android-debugger --device emulator-5554` provides an
explicit emulator-only smoke test after building `demoDebug`.

**Compose preview** builds the selected variant and opens a rendered image beside
the code. **Select preview…** switches between the default and large-text
annotations. Rendering uses downloaded Google tooling, JDK 21, and the selected
variant's resources; Android Studio and a running device are unnecessary. Refresh
after code changes. Interactive previews and multi-value preview parameter
galleries are not implemented.
