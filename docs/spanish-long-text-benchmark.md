# Spanish long-text benchmark

Date: 2026-09-01

## Verdict

The detector is fast for message-sized Spanish text. It is not suitable for unrestricted articles in its current form.

A 285-byte input takes about 0.212 ms. A 232 KB article takes about 589 ms.

The detector warned on 206 of 1,000 full articles. This is a warning rate, not a false-positive rate.

## Corpus

The benchmark uses the [Wikimedia Spanish Wikipedia dataset](https://huggingface.co/datasets/wikimedia/wikipedia/tree/main/20231101.es).

The source provides cleaned Wikipedia articles. The dataset card lists CC BY-SA 3.0 and GFDL licenses.

The benchmark fetched the first 1,000 rows from the `20231101.es` configuration through the official dataset API.

| Measure | Value |
|---|---:|
| Documents | 1,000 |
| Text bytes | 17,565,599 |
| Minimum document | 112 bytes |
| Median document | 7,560 bytes |
| 95th percentile | 70,355 bytes |
| Maximum document | 232,270 bytes |

Corpus SHA-256: `32d344a40c32e2e344ca9ef4b9be8d8a6e099f54e80ba4d9ed92c126b2108507`

## Test method

1. The runner loaded the corpus before timing.
2. The runner loaded the Spanish HurtLex file once.
3. The runner built one `Detector` instance and reused it.
4. The runner completed three untimed warm-up passes.
5. The runner completed ten timed passes with rotated document order.

Each timed call used this product path:

```rust
detector
    .analyze(text, AnalysisContext::for_language("ES"))
    .nudge()
```

Criterion measured fixed document sizes. The runner used `black_box` for each input and result.

## Test system

| Component | Value |
|---|---|
| Computer | MacBook Pro |
| Processor | Apple M5 Pro, 18 cores |
| Memory | 48 GB |
| Operating system | macOS 26.6.2 |
| Architecture | arm64 |
| Rust | rustc 1.97.0 |
| Build | Release, thin LTO, one code generation unit |

## Startup time

| Phase | Time |
|---|---:|
| Parse the Spanish lexicon | 3.183 ms |
| Build three match indexes | 22.784 ms |
| First 280-character analysis | 0.443 ms |
| Total cold startup and first analysis | 26.410 ms |

The server should build the detector once. Each request should reuse it.

## Fixed-size latency

These values are Criterion estimates. The range is Criterion's 95% confidence interval.

| Input | Estimated latency | Range | Throughput |
|---|---:|---:|---:|
| 285 bytes | 0.212 ms | 0.208 to 0.215 ms | 1.282 MiB/s |
| 1,026 bytes | 0.593 ms | 0.587 to 0.603 ms | 1.649 MiB/s |
| 4,093 bytes | 2.712 ms | 2.431 to 2.919 ms | 1.439 MiB/s |
| 16,358 bytes | 9.686 ms | 9.505 to 9.962 ms | 1.611 MiB/s |
| 65,804 bytes | 76.158 ms | 74.829 to 77.528 ms | 0.824 MiB/s |
| 232,270 bytes | 588.550 ms | 579.040 to 598.740 ms | 0.376 MiB/s |

The runtime grows faster than the input after about 16 KB. Long inputs activate more token scans, matches, and feature bins.

## Full-corpus throughput

The timed workload processed 10,000 documents and 175.7 MB of Spanish text.

| Measure | Result |
|---|---:|
| Elapsed time | 220.784 seconds |
| Documents per second | 45.29 |
| Throughput | 0.759 MiB/s |
| Mean latency | 22.078 ms |
| Median latency | 5.871 ms |
| 95th percentile latency | 93.312 ms |
| 99th percentile latency | 267.836 ms |
| Maximum latency | 758.725 ms |

## Message-size control

The runner also tested the first 280 characters from each article. This set is only a length control.

| Measure | Result |
|---|---:|
| Documents | 1,000 |
| Timed calls | 10,000 |
| Documents per second | 4,172.87 |
| Mean latency | 0.240 ms |
| Median latency | 0.231 ms |
| 95th percentile latency | 0.260 ms |
| Maximum latency | 0.319 ms |
| Warnings | 0 of 1,000 |
| Highest score | 49 |

The 280-character set contains article openings. It does not represent user messages.

## Warning behavior

The detector warned on 206 of 1,000 full articles. The warning rate was 20.6%.

Wikipedia articles can quote abuse, describe violence, or contain identity terms. The corpus has no toxicity labels.

The highest-scoring documents were also among the longest. Several received a score of 100.

The sparse model learned from short comments. Full articles activate many more unique features than short messages.

Do not interpret the 20.6% warning rate as product accuracy.

## Synthetic toxic additions

The runner selected 100 full articles with no initial warning. It appended one toxic sentence to each article.

The detector warned on 90 of 100 synthetic variants.

Nine templates produced ten warnings from ten variants. `Que te jodan.` produced zero warnings from ten variants.

This set measures signal survival inside long text. It does not measure recall on natural toxic documents.

## Memory

The memory probe used the largest article. Its input size was 232,270 bytes.

The process reached a maximum resident set size of 36,536,320 bytes, or 34.8 MiB.

This value includes the binary, detector, article, temporary views, matches, evidence, and allocator state.

## Product conclusion

Keep the detector on message-sized inputs. A 4 KB input takes about 2.7 ms on the test computer.

Set an explicit input limit. Do not send unrestricted articles through the current request path.

If long-form input becomes a requirement, score bounded sentence windows. Do not score one complete article as one sparse feature set.
