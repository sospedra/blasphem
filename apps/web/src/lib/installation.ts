import manifest from "../../../../packages/javascript/package.json";

type CodeExample = {
  label: string;
  code: string;
  lang: "bash" | "typescript" | "swift" | "kotlin" | "python" | "go" | "rust";
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

await init({ locales: ["en", "es"], grawlix: true });

const verdict = judge("you are a stupid loser");
console.log(verdict);`;

export const INSTALLATIONS: readonly Installation[] = [
  {
    id: "browser", name: "Web", group: "Clients", detail: "Browser · WebAssembly",
    install: { label: "Install with pnpm", lang: "bash", code: "pnpm add blasphem" },
    example: { label: "Initialize once, judge each message", lang: "typescript", code: javascript },
    notes: ["The browser loads the engine and selected languages from jsDelivr by default.", "After initialization, checks run locally. Your message never leaves the device."],
    docs: "packages/javascript#where-the-bytes-come-from",
  },
  {
    id: "react-native", name: "React Native", group: "Clients", detail: "iOS & Android · Nitro Modules",
    install: { label: "Install the package and its peers", lang: "bash", code: "pnpm add @blasphem/react-native @blasphem/packs react-native-nitro-modules" },
    example: { label: "Initialize once, judge each message", lang: "typescript", code: javascript.replace('from "blasphem"', 'from "@blasphem/react-native"') },
    notes: [
      "Bundle manifest.json, en.pack, es.pack, en.detect, and es.detect from node_modules/@blasphem/packs/dist/.",
      "iOS: add a blasphem folder reference to the app target. Android: use android/app/src/main/assets/blasphem/.",
      "Install iOS pods and rebuild the native app. Expo requires a development build.",
    ],
    docs: "packages/react-native#packs-in-the-app-bundle",
  },
  {
    id: "swift", name: "Swift", group: "Clients", detail: "iOS 15.1+ & macOS 12+ · Swift Package Manager",
    install: { label: "Package.swift · add the dependency", lang: "swift", code: `.package(
  url: "https://github.com/sospedra/blasphem-swift.git",
  from: "${version}"
)` },
    setup: { label: "App target · add these products", lang: "swift", code: `.product(name: "Blasphem", package: "blasphem-swift"),
.product(name: "BlasphemPackEN", package: "blasphem-swift"),
.product(name: "BlasphemPackES", package: "blasphem-swift"),
.product(name: "BlasphemDetectEN", package: "blasphem-swift"),
.product(name: "BlasphemDetectES", package: "blasphem-swift"),` },
    example: { label: "Create the judge off the main thread", lang: "swift", code: `import Blasphem

let judge = try Judge(locales: ["en", "es"], grawlix: true)
let verdict = try judge.judge("you are a stupid loser")
print(verdict)` },
    notes: ["Link one Pack product and one Detect product per language. SwiftPM bundles their data.", "To omit language detection, remove the Detect products and set detectLanguage: false."],
    docs: "packages/apple",
  },
  {
    id: "android", name: "Android", group: "Clients", detail: "Kotlin · Android API 24+ · Maven Central",
    install: { label: "build.gradle.kts · dependencies", lang: "kotlin", code: `dependencies {
  implementation(platform("me.sospedra.blasphem:blasphem-bom:${version}"))
  implementation("me.sospedra.blasphem:blasphem")
  implementation("me.sospedra.blasphem:blasphem-pack-en")
  implementation("me.sospedra.blasphem:blasphem-pack-es")
  implementation("me.sospedra.blasphem:blasphem-detect-en")
  implementation("me.sospedra.blasphem:blasphem-detect-es")
}` },
    example: { label: "Create the judge off the main thread", lang: "kotlin", code: `import me.sospedra.blasphem.Judge
import me.sospedra.blasphem.JudgeOptions

val judge = Judge.create(
  context,
  JudgeOptions(locales = listOf("en", "es"), grawlix = true)
)
val verdict = judge.judge("you are a stupid loser")` },
    notes: ["Enable mavenCentral() in your project repositories. Gradle bundles the selected language assets.", "Remove the detect artifacts and set detectLanguage = false to omit detection."],
    docs: "packages/android",
  },
  {
    id: "node", name: "Node.js", group: "Servers", detail: "JavaScript & TypeScript · native engine, WASM fallback",
    install: { label: "Install the engine and language data", lang: "bash", code: "pnpm add blasphem @blasphem/packs" },
    example: { label: "Initialize once, judge each message", lang: "typescript", code: javascript },
    notes: ["Node reads the installed language packs. No asset URL is required.", "The package selects the native engine for your platform, with WebAssembly as a fallback."],
    docs: "packages/javascript#nextjs",
  },
  {
    id: "python", name: "Python", group: "Servers", detail: "Python 3.10+ · native Rust extension",
    install: { label: "Install with pip", lang: "bash", code: "pip install blasphem blasphem-packs" },
    example: { label: "Initialize once, judge each message", lang: "python", code: `import blasphem

blasphem.init(["en", "es"], grawlix=True)
verdict = blasphem.judge("you are a stupid loser")
print(verdict)` },
    notes: ["The extension reads the installed blasphem-packs data. Each check runs synchronously."],
    docs: "packages/python",
  },
  {
    id: "go", name: "Go", group: "Servers", detail: "Go 1.25+ · embedded WebAssembly · no cgo",
    install: { label: "Add the Go module", lang: "bash", code: "go get github.com/sospedra/blasphem/packages/go" },
    setup: { label: "Export language data during your build", lang: "bash", code: "pnpm add blasphem @blasphem/packs\npnpm exec blasphem-assets ./blasphem-data" },
    example: { label: "main.go", lang: "go", code: `package main

import (
  "fmt"
  "log"
  blasphem "github.com/sospedra/blasphem/packages/go"
)

func main() {
  err := blasphem.Init(blasphem.Options{
    Locales: []string{"en", "es"},
    Assets: "./blasphem-data",
    Grawlix: true,
  })
  if err != nil {
    log.Fatal(err)
  }
  defer blasphem.Close()
  fmt.Println(blasphem.Judge("you are a stupid loser"))
}` },
    notes: ["Ship blasphem-data beside your app, or supply an embedded fs.FS through Options.Packs.", "Node is only needed for the data export above. The Go runtime uses wazero."],
    docs: "packages/go#packs",
  },
  {
    id: "rust", name: "Rust", group: "Servers", detail: "Native · language data compiled into the crate",
    install: { label: "Add the crate", lang: "bash", code: "cargo add blasphem" },
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
    notes: ["Build one Judge and reuse it. The crate includes the language data."],
    docs: "#rust",
  },
  {
    id: "cli", name: "CLI", group: "Terminal", detail: "Native binary · command line or stdin",
    install: { label: "Run without a global install", lang: "bash", code: 'npx blasphem judge --locales en,es "hello there"' },
    example: { label: "Read one message per line", lang: "bash", code: 'printf \'hello there\\nyou are a stupid loser\\n\' | npx blasphem judge --locales en --no-detect --json' },
    notes: ["Exit codes: 0 means no warning, 1 means a warning, and 2 means an error.", "Add --grawlix to return masked text. Use --no-detect when the language is known."],
    docs: "#command-line",
  },
];
