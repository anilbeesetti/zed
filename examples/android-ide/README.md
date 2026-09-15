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

Use **Configure official Kotlin** in the Android tools panel. The pinned
`263.4702.0+android-1` build includes a source-built native importer patch for
selected dependency variants, exact library sources, and dynamic features, plus
semantic Compose completion and naming fixes. The installer verifies the upstream
archive, public source, build dependencies, and installed patch. Its manifest and
source license remain with the runtime.

Setup requests the selected variant's resources. A Gradle task-graph guard stops
the request before execution if it would compile Kotlin or Java; some resource
tasks pull in library compilation. Generation failures report degraded generated
symbol support while still configuring Kotlin. Gradle, dependency, resource,
manifest, and selected-variant changes refresh the managed backend automatically.
An unavailable selected variant pauses it until a valid variant is selected.
Explicit community fallback and disabled-server settings are preserved.

Library source and decompiled `jar:`/`jrt:` tabs support navigation, but their
diagnostics are currently ignored.

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

Required assertion failures exit nonzero. Use `--require-original-sources` and
`--require-compose` to require the source, nested-navigation, and Compose fixes
included in the patched installation.
`--variant fullRelease` also checks that the dependent library selects `release`;
the unmodified pinned server falls back to `greeting.debug`, which fails this gate.
The patched native importer passes both variants and original Compose source
navigation.
For a custom build type, pass both `--variant fullStaging` and
`--expected-library-variant release` to check the intended `matchingFallbacks`.

For another isolated Gradle project, `--model-only --module :app --variant debug`
checks import/index readiness and the exported model without editing documents.
Repeat `--expected-module <exact-exported-name>` for each required module, including
dynamic feature main/test variants. Module names are matched exactly against the
`active_modules` array in the report. Document and performance checks require the
sample fixture and cannot be combined with `--model-only`.

Add `--extended` to exercise references and rename across application, library,
Kotlin and Java files, overload preservation, diagnostic clearing, quick fixes,
organize imports, formatting, signature help, type/implementation navigation,
and Compose completion contexts. Refactoring edits remain in memory; the probe
checks every touched source file is unchanged on disk. `--require-compose` makes
the Compose naming, required-lambda, and semantic named-argument ordering checks
required and implies `--extended`.
Java-origin requests remain explicit checks: Kotlin-origin rename can update Java
usages even when the official server returns no rename or references from Java.

Add `--performance-samples 30` for 30 samples each of warm definition, immediate
unsaved Modifier completion, explicit and dot-triggered String completion, and
named arguments. Reports include every latency, p50, nearest-rank p95, failures,
hardware and instantaneous
process RSS. The first definition is a separate single observation. These are
headless request/response timings; they exclude editor input, buffer creation and
rendering. Matching Gradle/JDT/preview/editor processes can belong to another
workspace, so their RSS is not a controlled total-memory measurement. Use the
same output directory for persistent-index restart comparisons and preserve its
previous report before rerunning.
`--warmup diagnostics`, `--warmup document-symbols`, and `--warmup hover` each
issue one bounded request for the active file before the first definition and
report that warmup cost separately from indexing readiness. `--jfr` captures a
bounded Java Flight Recorder profile in the report directory; its overhead is
included in that run's timings.

This is a headless protocol check. Separate Zed client regressions cover
incremental completion acceptance, command edits, caret undo/redo, workspace-edit
failure reporting, and virtual-document ownership/lifecycle in
`crates/editor/src/editor_tests.rs` and `crates/project/tests/integration/lsp_store.rs`.
The client suite separately tests expired sessions and late responses after timeout.

## Real editor check

The ignored `test_real_kotlin_editor_completion_and_navigation` test uses the
actual server, a GPUI test window, and unsaved source edits in an isolated Gradle
project copy. Set `ZED_KOTLIN_CLIENT_PROBE_CONFIG` to an absolute JSON file containing
`root`, a root-relative Kotlin `source`, absolute report `output`, `warm_samples`
(at least 30 for percentile measurements), `completion_refinement: true`,
`require_compose: true`, and the normal Zed `binary` and `initialization_options`
objects for `kotlin-lsp`. Keep the server's
`--system-path`, `idea.config.path`, and `idea.log.path` under the report directory;
set `LSP_ANDROID_MODULE`, `LSP_ANDROID_VARIANT`, and project JDK in `binary.env`.
Use the same Gradle initialization options as **Configure official Kotlin**.
Prefix refinement deletes/retypes the completion prefix in the existing function;
without it, each sample replaces the probe function. Optional
`completion_warm_samples` overrides `warm_samples` for completion scenarios.

```sh
ZED_KOTLIN_CLIENT_PROBE_CONFIG=/absolute/path/probe.json \
  cargo test -p editor --features project/test-support,gpui/inspector \
  test_real_kotlin_editor_completion_and_navigation -- --ignored --nocapture
```

The test records first and warm navigation, displayed completion rank, acceptance,
imports, caret, and process RSS; it verifies source text remains unchanged on disk.
It excludes release rendering/GPU latency. Shared Gradle daemons are reported
separately, and JDT/preview are inactive; this is not a complete IDE memory budget.
Do not run builds or other performance probes during timing collection.

## Java, debugging, and preview

Run `script/install-android-kotlin`, `script/install-android-debugger`, and
`script/install-android-preview` once before launching the IDE. **Configure Java**
imports the selected variant into JDT LS; repeat Java setup after changing variants
or dependencies. Managed official Kotlin setup refreshes automatically.

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
