import manifest from "../../../../packages/javascript/package.json";

type CodeExample = {
  label: string;
  code: string;
  lang: "bash" | "typescript" | "swift" | "kotlin" | "python" | "go" | "rust" | "json" | "toml";
};

export type Installation = {
  id: string;
  name: string;
  group: "Clients" | "Servers" | "Terminal";
  detail: string;
  install: CodeExample;
  setup?: CodeExample;
  example: CodeExample;
  notes: readonly string[];
  docs: string;
};

const version = manifest.version;
const javascript = `import { init, judge } from "blasphem";

await init({ grawlix: true });

const verdict = judge("you are a stupid loser");
console.log(verdict);`;
const configuration: CodeExample = {
  label: "package.json · select once", lang: "json", code: JSON.stringify({
    blasphem: { locales: ["en", "es"], assets: "bundled", detectLanguage: true },
  }, null, 2),
};

export const INSTALLATIONS: readonly Installation[] = [
  {
    id: "browser", name: "Web", group: "Clients", detail: "Browser · WebAssembly",
    install: { label: "Install with pnpm", lang: "bash", code: "pnpm add blasphem" },
    setup: configuration,
    example: { label: "Initialize once, judge each message", lang: "typescript", code: javascript },
    notes: ["Register blasphem/vite, or publish assets with blasphem-assets and load its config.js.", "Bundled delivery is the default. Set assets to remote for pinned jsDelivr downloads.", "Use locales: all for every release locale. Message checks stay local."],
    docs: "packages/javascript#browser-assets",
  },
  {
    id: "react-native", name: "React Native", group: "Clients", detail: "iOS & Android · Nitro Modules",
    install: { label: "Install the package and its Nitro peer", lang: "bash", code: "pnpm add @blasphem/react-native react-native-nitro-modules" },
    setup: configuration,
    example: { label: "Initialize once, judge each message", lang: "typescript", code: javascript.replace('from "blasphem"', 'from "@blasphem/react-native"') },
    notes: [
      "CocoaPods and Gradle copy only selected data automatically. Set assets to remote for persistent downloads.",
      "Expo applications register @blasphem/react-native/app.plugin in app.json.",
      "Install iOS pods and rebuild the native app. Expo requires a development build.",
    ],
    docs: "packages/react-native#bundle-language-data",
  },
  {
    id: "swift", name: "Swift", group: "Clients", detail: "iOS 15.1+ & macOS 12+ · Swift Package Manager",
    install: { label: "Package.swift · add the dependency", lang: "swift", code: `.package(
  url: "https://github.com/sospedra/blasphem-swift.git",
  exact: "${version}"
)` },
    setup: { label: "blasphem.json · project root", lang: "json", code: '{\n  "locales": ["en", "es"],\n  "assets": "bundled",\n  "detectLanguage": true\n}' },
    example: { label: "Create the configured judge", lang: "swift", code: `import Blasphem

let judge = try await Judge.create(grawlix: true)
let verdict = try judge.judge("you are a stupid loser")
print(verdict)` },
    notes: ["Add the Blasphem library product and attach the BlasphemAssets build plugin to your target.", "The plugin supports SwiftPM and Xcode application targets. It reads blasphem.json automatically.", "Use locales: all for every release locale. Remote delivery bundles no language data."],
    docs: "packages/apple",
  },
  {
    id: "android", name: "Android", group: "Clients", detail: "Kotlin · Android API 24+ · Maven Central",
    install: { label: "build.gradle.kts · plugin", lang: "kotlin", code: `plugins {
  id("me.sospedra.blasphem") version "${version}"
}` },
    setup: { label: "build.gradle.kts · selection", lang: "kotlin", code: `blasphem {
  locales.set(listOf("en", "es")) // Also accepts "all".
  assets.set("bundled")
  detectLanguage.set(true)
}` },
    example: { label: "Inside a coroutine", lang: "kotlin", code: `import me.sospedra.blasphem.Judge

val judge = Judge.create(context)
val verdict = judge.judge("you are a stupid loser")` },
    notes: ["Enable mavenCentral() for dependencies and plugins. The plugin adds exact engine and data dependencies.", "Set assets to remote for private-files downloads. Set detectLanguage to false to omit detection files."],
    docs: "packages/android",
  },
  {
    id: "node", name: "Node.js", group: "Servers", detail: "JavaScript & TypeScript · native engine, WASM fallback",
    install: { label: "Install the library", lang: "bash", code: "pnpm add blasphem" },
    setup: configuration,
    example: { label: "Initialize once, judge each message", lang: "typescript", code: javascript },
    notes: ["Node reads package.json without a frontend build. Its exact data dependency installs automatically.", "Node uses a native engine with a local WebAssembly fallback. Remote delivery is unsupported.", "Use blasphem-export --locales en,es --output ./vendor for a reduced deployment."],
    docs: "packages/javascript#node-assets",
  },
  {
    id: "python", name: "Python", group: "Servers", detail: "Python 3.10+ · native Rust extension",
    install: { label: "Install with pip", lang: "bash", code: "python -m pip install blasphem" },
    example: { label: "Initialize once, judge each message", lang: "python", code: `import blasphem

blasphem.init(["en", "es"], grawlix=True)
verdict = blasphem.judge("you are a stupid loser")
print(verdict)` },
    notes: ["The exact internal data dependency installs automatically. init also accepts all.", "Use python -m blasphem export --locales en,es --output ./vendor for a reduced deployment."],
    docs: "packages/python",
  },
  {
    id: "go", name: "Go", group: "Servers", detail: "Go 1.25+ · embedded WebAssembly · no cgo",
    install: { label: "Add the Go module", lang: "bash", code: "go get github.com/sospedra/blasphem/packages/go/v2" },
    example: { label: "main.go", lang: "go", code: `package main

import (
  "fmt"
  "log"
  blasphem "github.com/sospedra/blasphem/packages/go/v2"
  "github.com/sospedra/blasphem/packages/go/v2/locales/en"
  "github.com/sospedra/blasphem/packages/go/v2/locales/es"
)

func main() {
  err := blasphem.Init(blasphem.Options{
    Locales: []blasphem.Locale{en.Locale, es.Locale},
    Grawlix: true,
  })
  if err != nil {
    log.Fatal(err)
  }
  defer blasphem.Close()
  fmt.Println(blasphem.Judge("you are a stupid loser"))
}` },
    notes: ["Each locale subpackage embeds its own data. The root package references no full catalog.", "Import locales/all and use all.Locales for every release locale. Custom filesystem sources remain available."],
    docs: "packages/go",
  },
  {
    id: "rust", name: "Rust", group: "Servers", detail: "Native · language data compiled into the crate",
    install: { label: "Cargo.toml · selected data", lang: "toml", code: `[dependencies.blasphem]
git = "https://github.com/sospedra/blasphem"
default-features = false
features = ["embedded", "language-detection", "en", "es"]` },
    example: { label: "src/main.rs", lang: "rust", code: `use blasphem::{Judge, JudgeOptions, Language};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let judge = Judge::new(JudgeOptions {
        locales: vec![Language::En, Language::Es],
        detect_language: true,
        grawlix: true,
    })?;
    let verdict = judge.judge("you are a stupid loser");
    println!("{:?}", verdict);
    Ok(())
}` },
    notes: ["Locale features gate compiled models, lexicons, rules, and detection data.", "Default features include all locales. Runtime filters cannot remove compiled bytes."],
    docs: "crates/blasphem",
  },
  {
    id: "cli", name: "CLI", group: "Terminal", detail: "Native binary · command line or stdin",
    install: { label: "Run without a global install", lang: "bash", code: 'npx blasphem judge --locales en,es "hello there"' },
    example: { label: "Read one message per line", lang: "bash", code: 'printf \'hello there\\nyou are a stupid loser\\n\' | npx blasphem judge --locales en --no-detect --json' },
    notes: ["Exit codes: 0 means no warning, 1 means a warning, and 2 means an error.", "Prebuilt binaries contain all languages. --locales all loads every release locale.", "Add --grawlix for masked text. --no-detect omits language detection loading."],
    docs: "packages/cli",
  },
];
