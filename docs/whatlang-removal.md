# whatlang removal

Date: 2026-09-03

Tree: branch `development` at `ba50ca6`. Measurements ran against charabia 0.9.9 with the patch below applied through `[patch.crates-io]`, before the confusable-view fix in `ba50ca6` landed.

## Verdict

Remove whatlang's trigram detector from the charabia build. Arabic gets 5.8x faster, the Node p95 tail drops under the 1 ms gate, and no verdict changes.

## What whatlang is

`whatlang` is a trigram language identifier that ships a profile table for about 70 languages. Blasphem does not use it. `charabia`, the tokenizer behind every `.tokenize()` call in `src/text.rs` and `src/detector.rs`, requires it unconditionally.

charabia calls two functions. `detect_script` classifies characters by script; it is small and every normalizer keys on it. `detect_lang` is the trigram detector. It runs only when a script has two or more specialized segmenters (`charabia/src/segmenter/mod.rs:280-299`). With Blasphem's feature set the map holds three entries: Latin with no language, Arabic with Arabic, and Arabic with Persian. So the trigram detector runs on every Arabic-script call, and its answer feeds `PersianNormalizer`, which folds Persian letter variants when it labels the text Persian (`normalizer/persian.rs:22-26`).

Blasphem already routes text with its own detector in `blasphem-language`. The binary carries two.

## What we win

| Measure | Before | After |
|---|---|---|
| Arabic, 280 scalars, Node p50 | 1.69 to 1.74 ms | 0.30 ms |
| All fixtures, 280 scalars, Node p95 | 1.18 to 1.19 ms | 0.40 ms |
| All fixtures, 4,096 bytes, Node p95 | 7.7 to 8.0 ms | 3.3 to 3.4 ms |
| Cold init, Node | 0.52 s | 0.38 to 0.39 s |
| Full wasm after wasm-bindgen | 10.76 MB | 10.51 MB |
| Full wasm gzip | 5.65 MB | 5.59 MB |

Every language other than Arabic moved by less than the run-to-run noise. The whole p95 tail of the current build was Arabic paying for a trigram pass per call.

## What we lose

Arabic text that whatlang labels Persian is normalized with Persian letter folding today. HurtLex Arabic lemmas use Arabic letters, so folded tokens stop matching. Without the detector the folding never happens.

Parity over 90 fixtures plus 5,000 corpus rows per language, 74,398 rows, comparing verdict, score, locale, and grawlix: 74,352 identical, 46 different, all Arabic. Zero verdicts flipped. 18 rows score higher, 28 only mask an extra span. Of the 46, 30 were the Algeria false positive that `ba50ca6` has since removed, and 19 were toxic rows that gained real hits such as الارهابيين, زنديق, and الزانية.

The other cost is a fork to maintain until charabia accepts a feature flag.

## The patch

Against charabia 0.9.9. `detect_script` stays, so `whatlang` remains a dependency; the linker drops the trigram table because nothing references `detect_lang` any more. Arabic keeps `ArabicSegmenter` through a single map entry, so the language branch in `segmenter()` is never taken.

```diff
--- a/src/detection/mod.rs
+++ b/src/detection/mod.rs
@@ -1,5 +1,4 @@
 pub use script_language::{Language, Script};
-use whatlang::Detector;
 
 // file copy pasted from whatlang.
 #[allow(dead_code)]
@@ -30,7 +29,7 @@
             None => match self.allow_list {
                 Some([unique_language]) => Some(*unique_language),
                 None if Self::detect_script(inner) == Script::Latin => None,
-                _otherwise => Self::detect_lang(inner, self.allow_list),
+                _otherwise => None,
             },
         };
 
@@ -43,16 +42,6 @@
         whatlang::detect_script(text).map(Script::from).unwrap_or_default()
     }
 
-    /// detect lang with whatlang
-    /// if no language is detected, return Language::Other
-    fn detect_lang(text: &str, allow_list: Option<&[Language]>) -> Option<Language> {
-        let detector = allow_list
-            .map(|allow_list| allow_list.iter().map(|lang| (*lang).into()).collect())
-            .map(Detector::with_allowlist)
-            .unwrap_or_default();
-
-        detector.detect_lang(text).map(Language::from)
-    }
 }
 
 pub trait Detect<'o, 'al> {
--- a/src/segmenter/mod.rs
+++ b/src/segmenter/mod.rs
@@ -77,9 +77,8 @@
         #[cfg(feature = "khmer")]
         ((Script::Khmer, Some(Language::Khm)), Box::new(KhmerSegmenter) as Box<dyn Segmenter>),
         // arabic segmenter
-        ((Script::Arabic, Some(Language::Ara)), Box::new(ArabicSegmenter) as Box<dyn Segmenter>),
+        ((Script::Arabic, None), Box::new(ArabicSegmenter) as Box<dyn Segmenter>),
         // persian segmenter
-        ((Script::Arabic, Some(Language::Pes)), Box::new(ArabicSegmenter) as Box<dyn Segmenter>),
         // german segmenter
         #[cfg(feature = "german-segmentation")]
         ((Script::Latin, Some(Language::Deu)), Box::new(GermanSegmenter) as Box<dyn Segmenter>),
```

## How to land it

1. Fork `https://github.com/meilisearch/charabia` at the 0.9.9 tag and apply the patch.
2. Point the workspace at the fork by git revision in `[patch.crates-io]`.
3. Rerun the parity harness on top of `ba50ca6`. The 46-row result above predates that commit.
4. Rerun `cargo test --workspace --locked`, the validation evaluation, and the behavior panels. The native suites of `blasphem` and `blasphem-language` passed against the patch: 24 suites, 259 tests.
5. Open the upstream pull request for a feature flag that compiles the trigram path out, and drop the fork when it merges.

## Evidence

Sizes are post-wasm-bindgen bytes of `blasphem_bg.wasm`. Timings come from a Node 24 harness over `tests/fixtures/benchmark/messages.jsonl`, all 15 locales loaded, detection on, 500 samples per short fixture and 100 per long one, two rounds in reversed order. An identical binary run twice bounded the noise at about 1 percent.
