# Turkish recall report

## Decision

Freeze `TurkishChar35V3` with class-weighted L2 logistic training.

The profile uses character 3-grams through 5-grams within tokens.
The compiler uses inverse-frequency weights for the two classes.
The runtime artifact remains a 131,112-byte `SparseV2` file.
No corpus, lexicon, or behavior fixture changed.

## Audit

The baseline had 12 false positives and 788 false negatives.
The audit covered every false positive.
It sampled 200 false negatives across score and length bands.
Each audit row records scores, length, and lexicon hits.
The full record is [`reports/tr-recall-validation-audit.json`](../reports/tr-recall-validation-audit.json).

Ten false positives had conservative lexicon hits.
Thirty sampled false negatives had conservative lexicon hits.
The lexicon evidence was too sparse for a lexicon-only fix.

No native reviewer was available.
Therefore, the audit leaves all cause labels unassigned.
No unreviewed text entered development.

## Validation ablations

Evidence status: tuned validation evidence.

Each ablation used the same validation rows and gates.
The test split stayed sealed during candidate selection.

| candidate | TP | FN | FP | TN | decision |
|---|---:|---:|---:|---:|---|
| `WordChar35V2`, Bernoulli control | 116 | 788 | 12 | 3,691 | reject |
| token character 3-5, Bernoulli | 123 | 781 | 13 | 3,690 | partial |
| token character 3-9, Bernoulli | 109 | 795 | 12 | 3,691 | reject |
| ASCII-folded character 3-5, Bernoulli | 112 | 792 | 12 | 3,691 | reject |
| `WordChar35V2`, class-weighted L2 logistic | 200 | 704 | 22 | 3,681 | partial |
| token character 3-5, class-weighted L2 logistic | 227 | 677 | 25 | 3,678 | freeze |
| token character 3-9, class-weighted L2 logistic | 227 | 677 | 25 | 3,678 | reject as redundant |

The frozen candidate raises validation recall by 12.28 points.
Validation precision is 90.08%.
The false-warning rate is 0.68%.
All 15 language gates pass.

## Paired validation summary

The frozen candidate changed 204 validation verdicts.

| label | direction | rows |
|---|---|---:|
| toxic | safe to flag | 141 |
| toxic | flag to safe | 30 |
| clean | safe to flag | 23 |
| clean | flag to safe | 10 |

The net paired gain is 111 true positives.
The net paired cost is 13 false positives.

## Final evidence

Evidence status: held-out test evidence.

The final combined benchmark measured the frozen candidate.
Its run is `benchmark/runs/0a8d974.json`.

| split | TP | FN | FP | TN | recall | precision | false-warning rate |
|---|---:|---:|---:|---:|---:|---:|---:|
| validation | 227 | 677 | 25 | 3,678 | 25.11% | 90.08% | 0.68% |
| test | 173 | 543 | 19 | 2,793 | 24.16% | 90.10% | 0.68% |

The baseline test recall was 13.27%.
The frozen test recall gains 10.89 percentage points.
The 360 behavior cases pass.
The 60 CLI smoke cases pass.

The Turkish artifact SHA-256 is
`d99e4a38451a36e8d6d4a3e8589dfa1aea676bbfc6b0ee11dc291804a3be8dfe`.

Two local compiles produced byte-identical Turkish artifacts.
The canonical build platform remains the cross-platform authority.

## Verification

```text
$ cargo run --release --locked -p blasphem-train -- corpus-verify --corpus-root corpus --evaluation-lock resources/datasets/evaluation-lock-v1.json
status=verified languages=15 rows=240879

$ cargo test --workspace --all-targets --locked
exit code: 0

$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo]

$ cargo fmt --all -- --check
exit code: 0

$ cargo run --release --locked -p blasphem-train -- reproduce --skip-browser
status=reproduced steps=8
```

## Paired validation changes

| source ID | label | direction | baseline score | frozen score |
|---|---|---|---:|---:|
| `TR:2239f24a7d64` | clean | `safe_to_flag` | 37 | 51 |
| `TR:41041da3cfb3` | clean | `flag_to_safe` | 63 | 30 |
| `TR:978612b40c83` | clean | `safe_to_flag` | 49 | 51 |
| `TR:23230b4f6c13` | clean | `safe_to_flag` | 23 | 52 |
| `TR:9ead518a1f20` | clean | `safe_to_flag` | 31 | 50 |
| `TR:d282af1a928e` | clean | `safe_to_flag` | 40 | 52 |
| `TR:89b834a644a2` | clean | `safe_to_flag` | 43 | 87 |
| `TR:c7780ea19fef` | clean | `safe_to_flag` | 21 | 51 |
| `TR:e8c2da19f79d` | clean | `safe_to_flag` | 36 | 58 |
| `TR:2588e42ab367` | clean | `safe_to_flag` | 33 | 68 |
| `TR:4da4de528550` | clean | `flag_to_safe` | 50 | 30 |
| `TR:2f91fadcd4cc` | clean | `safe_to_flag` | 46 | 58 |
| `TR:26bf12e5b6ae` | clean | `safe_to_flag` | 30 | 50 |
| `TR:fdc24067923d` | clean | `safe_to_flag` | 39 | 59 |
| `TR:37a566408163` | clean | `flag_to_safe` | 68 | 37 |
| `TR:c4ade43bbf2c` | clean | `safe_to_flag` | 36 | 52 |
| `TR:7eb75a864e25` | clean | `safe_to_flag` | 15 | 54 |
| `TR:c9851d12c540` | clean | `safe_to_flag` | 24 | 51 |
| `TR:cd52c4b50ac2` | clean | `flag_to_safe` | 57 | 49 |
| `TR:7e55e14da547` | clean | `flag_to_safe` | 56 | 30 |
| `TR:9efd26181923` | clean | `safe_to_flag` | 44 | 51 |
| `TR:8b32aeb91150` | clean | `safe_to_flag` | 49 | 69 |
| `TR:f3a961a903f7` | clean | `flag_to_safe` | 54 | 33 |
| `TR:4009bee3980d` | clean | `safe_to_flag` | 49 | 55 |
| `TR:45621fbb83f0` | clean | `safe_to_flag` | 26 | 50 |
| `TR:8dcb7f3b25f9` | clean | `flag_to_safe` | 51 | 39 |
| `TR:73ab8940da9e` | clean | `safe_to_flag` | 19 | 54 |
| `TR:730d2aea4021` | clean | `flag_to_safe` | 51 | 30 |
| `TR:d93cf46e4ed9` | clean | `safe_to_flag` | 37 | 62 |
| `TR:2564e8dbddd3` | clean | `flag_to_safe` | 50 | 47 |
| `TR:701e8649c03d` | clean | `safe_to_flag` | 46 | 53 |
| `TR:8c97c1acdd3a` | clean | `flag_to_safe` | 61 | 46 |
| `TR:304f6b697c63` | clean | `safe_to_flag` | 30 | 53 |
| `TR:54e6dcd0a823` | toxic | `safe_to_flag` | 7 | 52 |
| `TR:06c6f83ae525` | toxic | `safe_to_flag` | 24 | 55 |
| `TR:bfb7e5112527` | toxic | `safe_to_flag` | 32 | 59 |
| `TR:d1876c61d7a6` | toxic | `flag_to_safe` | 68 | 42 |
| `TR:378a014a4279` | toxic | `flag_to_safe` | 54 | 48 |
| `TR:280036bc4fce` | toxic | `safe_to_flag` | 7 | 50 |
| `TR:84fb10b7b424` | toxic | `flag_to_safe` | 55 | 40 |
| `TR:2399be17c3c7` | toxic | `safe_to_flag` | 38 | 53 |
| `TR:569d9a5a82cc` | toxic | `safe_to_flag` | 33 | 69 |
| `TR:f85505a35859` | toxic | `flag_to_safe` | 51 | 3 |
| `TR:ee7e5cbb3d9c` | toxic | `safe_to_flag` | 41 | 93 |
| `TR:558c1b3036b9` | toxic | `flag_to_safe` | 64 | 30 |
| `TR:5a3629a9df29` | toxic | `safe_to_flag` | 45 | 87 |
| `TR:bc4002c2bb6c` | toxic | `safe_to_flag` | 41 | 64 |
| `TR:c127ba3285ee` | toxic | `safe_to_flag` | 45 | 59 |
| `TR:3cdbd017518b` | toxic | `safe_to_flag` | 39 | 82 |
| `TR:578effc53584` | toxic | `safe_to_flag` | 12 | 61 |
| `TR:ddaaabd7ad9d` | toxic | `safe_to_flag` | 49 | 62 |
| `TR:246dfc82bd47` | toxic | `safe_to_flag` | 44 | 51 |
| `TR:e55de2dca35b` | toxic | `safe_to_flag` | 44 | 58 |
| `TR:4a3689604caf` | toxic | `safe_to_flag` | 31 | 75 |
| `TR:7891b45d0c64` | toxic | `flag_to_safe` | 54 | 19 |
| `TR:1a8f03e58e29` | toxic | `safe_to_flag` | 10 | 64 |
| `TR:6f77166afc8b` | toxic | `safe_to_flag` | 49 | 57 |
| `TR:069b1c63d6e7` | toxic | `safe_to_flag` | 19 | 59 |
| `TR:5790d9a9623c` | toxic | `safe_to_flag` | 11 | 54 |
| `TR:49a3ff6191b3` | toxic | `safe_to_flag` | 44 | 64 |
| `TR:9db520d44f20` | toxic | `safe_to_flag` | 39 | 53 |
| `TR:adac8925c45d` | toxic | `safe_to_flag` | 38 | 51 |
| `TR:3da107ed7d01` | toxic | `safe_to_flag` | 40 | 66 |
| `TR:3e7f2edaf83f` | toxic | `flag_to_safe` | 66 | 30 |
| `TR:290ccf101ebd` | toxic | `safe_to_flag` | 21 | 56 |
| `TR:2e52ee15955b` | toxic | `safe_to_flag` | 46 | 59 |
| `TR:a844643c8935` | toxic | `safe_to_flag` | 37 | 54 |
| `TR:d8c70c84bca5` | toxic | `safe_to_flag` | 43 | 66 |
| `TR:d3ee57024190` | toxic | `safe_to_flag` | 31 | 52 |
| `TR:e198a179beef` | toxic | `safe_to_flag` | 46 | 60 |
| `TR:d256895e916e` | toxic | `safe_to_flag` | 35 | 62 |
| `TR:4874847fb1c1` | toxic | `safe_to_flag` | 39 | 51 |
| `TR:2da573c3f4fe` | toxic | `safe_to_flag` | 44 | 67 |
| `TR:9e6803e5de47` | toxic | `flag_to_safe` | 65 | 30 |
| `TR:adff1ab76c2d` | toxic | `safe_to_flag` | 28 | 50 |
| `TR:dafac1a0276a` | toxic | `safe_to_flag` | 42 | 54 |
| `TR:37f0c29e35eb` | toxic | `safe_to_flag` | 37 | 56 |
| `TR:65853ea37cba` | toxic | `safe_to_flag` | 23 | 51 |
| `TR:a5a606a9d8c6` | toxic | `safe_to_flag` | 32 | 52 |
| `TR:b2829ede1072` | toxic | `flag_to_safe` | 50 | 44 |
| `TR:faf9e799c6cd` | toxic | `flag_to_safe` | 68 | 46 |
| `TR:f2b76783001a` | toxic | `safe_to_flag` | 39 | 50 |
| `TR:f975a389f216` | toxic | `safe_to_flag` | 43 | 51 |
| `TR:c1b12621222b` | toxic | `flag_to_safe` | 54 | 30 |
| `TR:7383dbe5c81d` | toxic | `safe_to_flag` | 48 | 55 |
| `TR:d67c743d1dfe` | toxic | `safe_to_flag` | 46 | 73 |
| `TR:a94e4a1c62b4` | toxic | `safe_to_flag` | 22 | 58 |
| `TR:056118f441a8` | toxic | `safe_to_flag` | 37 | 67 |
| `TR:53997a35ba54` | toxic | `flag_to_safe` | 52 | 47 |
| `TR:fa3302f9383a` | toxic | `safe_to_flag` | 42 | 56 |
| `TR:056b4175ea8b` | toxic | `safe_to_flag` | 43 | 50 |
| `TR:8dcbe1ea0265` | toxic | `safe_to_flag` | 38 | 58 |
| `TR:44d9cab17cf5` | toxic | `safe_to_flag` | 21 | 53 |
| `TR:63a46549434a` | toxic | `safe_to_flag` | 37 | 52 |
| `TR:8ccc23c3eaad` | toxic | `safe_to_flag` | 22 | 55 |
| `TR:9617d1bd579f` | toxic | `safe_to_flag` | 40 | 61 |
| `TR:a46e2ec262e6` | toxic | `safe_to_flag` | 31 | 51 |
| `TR:06f7f5929bfa` | toxic | `safe_to_flag` | 25 | 76 |
| `TR:2289d82d0265` | toxic | `safe_to_flag` | 42 | 53 |
| `TR:2f4629f4c777` | toxic | `safe_to_flag` | 39 | 64 |
| `TR:41ac5cc5288c` | toxic | `safe_to_flag` | 48 | 57 |
| `TR:2330774901f3` | toxic | `safe_to_flag` | 41 | 57 |
| `TR:8f97987abc1d` | toxic | `safe_to_flag` | 6 | 51 |
| `TR:b1a13413aea0` | toxic | `safe_to_flag` | 31 | 52 |
| `TR:095cdde64a7c` | toxic | `safe_to_flag` | 47 | 53 |
| `TR:d79185095531` | toxic | `safe_to_flag` | 45 | 71 |
| `TR:885fce895108` | toxic | `safe_to_flag` | 9 | 53 |
| `TR:e386c00e1e8f` | toxic | `safe_to_flag` | 10 | 59 |
| `TR:6378ff75cc90` | toxic | `safe_to_flag` | 39 | 53 |
| `TR:20a9204907cb` | toxic | `safe_to_flag` | 37 | 60 |
| `TR:795259f152e3` | toxic | `safe_to_flag` | 30 | 63 |
| `TR:e01d814928fa` | toxic | `safe_to_flag` | 49 | 59 |
| `TR:98cd31117ba4` | toxic | `safe_to_flag` | 20 | 53 |
| `TR:f4298b84bba6` | toxic | `safe_to_flag` | 12 | 51 |
| `TR:c94d93046e8b` | toxic | `safe_to_flag` | 12 | 67 |
| `TR:5d2cdb43e3e0` | toxic | `safe_to_flag` | 48 | 56 |
| `TR:426edea3475a` | toxic | `safe_to_flag` | 33 | 50 |
| `TR:6a23c417841a` | toxic | `safe_to_flag` | 32 | 87 |
| `TR:df82c713fe66` | toxic | `safe_to_flag` | 48 | 55 |
| `TR:df4e53b784eb` | toxic | `safe_to_flag` | 38 | 63 |
| `TR:052176d722a1` | toxic | `safe_to_flag` | 35 | 59 |
| `TR:366a808dcb55` | toxic | `safe_to_flag` | 38 | 81 |
| `TR:0a1b1be49c3a` | toxic | `safe_to_flag` | 47 | 61 |
| `TR:111a0f9f50bf` | toxic | `safe_to_flag` | 44 | 64 |
| `TR:e260031cc043` | toxic | `safe_to_flag` | 23 | 53 |
| `TR:cd511d309740` | toxic | `safe_to_flag` | 21 | 61 |
| `TR:0611452f8ffe` | toxic | `safe_to_flag` | 34 | 52 |
| `TR:73662b5a6fcc` | toxic | `safe_to_flag` | 49 | 50 |
| `TR:84f161651d06` | toxic | `flag_to_safe` | 58 | 31 |
| `TR:b5ce1206040f` | toxic | `safe_to_flag` | 19 | 54 |
| `TR:8d2397650b4e` | toxic | `safe_to_flag` | 44 | 54 |
| `TR:0edbc96920f7` | toxic | `flag_to_safe` | 52 | 30 |
| `TR:40c820d945f2` | toxic | `safe_to_flag` | 37 | 75 |
| `TR:58edb6684dc7` | toxic | `safe_to_flag` | 22 | 61 |
| `TR:e0551b2aa72a` | toxic | `safe_to_flag` | 46 | 51 |
| `TR:571770f1dfa2` | toxic | `safe_to_flag` | 34 | 65 |
| `TR:fc0fb6479327` | toxic | `safe_to_flag` | 42 | 54 |
| `TR:99fc69c5efd1` | toxic | `safe_to_flag` | 48 | 93 |
| `TR:69dd41bcdb48` | toxic | `safe_to_flag` | 45 | 98 |
| `TR:c0280b6476c3` | toxic | `safe_to_flag` | 48 | 56 |
| `TR:a43d03047a24` | toxic | `safe_to_flag` | 39 | 51 |
| `TR:1ff18b8f606d` | toxic | `safe_to_flag` | 25 | 53 |
| `TR:721f7dc16516` | toxic | `safe_to_flag` | 49 | 94 |
| `TR:f9d28cb5b6de` | toxic | `safe_to_flag` | 42 | 57 |
| `TR:b7bdfdc6bafd` | toxic | `safe_to_flag` | 12 | 50 |
| `TR:4d58c3f9281a` | toxic | `safe_to_flag` | 35 | 55 |
| `TR:0f4a11083275` | toxic | `safe_to_flag` | 25 | 54 |
| `TR:6a38b8d72fc7` | toxic | `flag_to_safe` | 58 | 49 |
| `TR:7b86263cd8a0` | toxic | `flag_to_safe` | 57 | 30 |
| `TR:8e5a3dd12323` | toxic | `safe_to_flag` | 30 | 62 |
| `TR:7c1f2555e355` | toxic | `safe_to_flag` | 10 | 62 |
| `TR:a40d905afd08` | toxic | `flag_to_safe` | 71 | 44 |
| `TR:7df4afac634d` | toxic | `safe_to_flag` | 8 | 53 |
| `TR:42e3d36b8377` | toxic | `safe_to_flag` | 9 | 57 |
| `TR:895a3eeddf96` | toxic | `flag_to_safe` | 62 | 43 |
| `TR:4cc765610567` | toxic | `safe_to_flag` | 31 | 56 |
| `TR:2e8211c93899` | toxic | `safe_to_flag` | 48 | 68 |
| `TR:fef63781b910` | toxic | `safe_to_flag` | 15 | 62 |
| `TR:1f1508d294b5` | toxic | `safe_to_flag` | 39 | 57 |
| `TR:69dde6030e05` | toxic | `safe_to_flag` | 45 | 71 |
| `TR:73e7e2ceefb6` | toxic | `safe_to_flag` | 42 | 68 |
| `TR:83acd830b813` | toxic | `safe_to_flag` | 46 | 65 |
| `TR:45d62f201053` | toxic | `flag_to_safe` | 55 | 38 |
| `TR:ab53c8fa6a74` | toxic | `flag_to_safe` | 55 | 47 |
| `TR:09eb456b4d86` | toxic | `flag_to_safe` | 59 | 32 |
| `TR:0c67e233b0cf` | toxic | `flag_to_safe` | 50 | 38 |
| `TR:4e42f9d78fb8` | toxic | `safe_to_flag` | 41 | 63 |
| `TR:b570031f00f6` | toxic | `safe_to_flag` | 14 | 67 |
| `TR:707aca38e690` | toxic | `safe_to_flag` | 47 | 56 |
| `TR:ab9012d354a4` | toxic | `flag_to_safe` | 70 | 40 |
| `TR:a23312099836` | toxic | `safe_to_flag` | 23 | 50 |
| `TR:fa5f30a8aee8` | toxic | `flag_to_safe` | 51 | 44 |
| `TR:4574425d7cc5` | toxic | `safe_to_flag` | 15 | 50 |
| `TR:2bab5f4cb706` | toxic | `flag_to_safe` | 60 | 41 |
| `TR:eeff2d124dff` | toxic | `safe_to_flag` | 39 | 50 |
| `TR:df5637d2c7d9` | toxic | `flag_to_safe` | 74 | 45 |
| `TR:d1c6513a1045` | toxic | `safe_to_flag` | 49 | 62 |
| `TR:ae6b0c3f9b18` | toxic | `safe_to_flag` | 35 | 69 |
| `TR:8ca363b1db44` | toxic | `safe_to_flag` | 49 | 57 |
| `TR:f0d022e284d0` | toxic | `safe_to_flag` | 45 | 100 |
| `TR:b1faf4af2107` | toxic | `safe_to_flag` | 40 | 51 |
| `TR:9c056b8ef436` | toxic | `safe_to_flag` | 17 | 50 |
| `TR:fc7831a74258` | toxic | `flag_to_safe` | 62 | 49 |
| `TR:e93ebdc3cb4e` | toxic | `safe_to_flag` | 48 | 53 |
| `TR:46a2e5a4dbc2` | toxic | `safe_to_flag` | 8 | 55 |
| `TR:c3b585d05f57` | toxic | `safe_to_flag` | 14 | 51 |
| `TR:4658ba086ebb` | toxic | `safe_to_flag` | 23 | 52 |
| `TR:6418e8d599c3` | toxic | `flag_to_safe` | 51 | 30 |
| `TR:6a1284d88da0` | toxic | `safe_to_flag` | 46 | 51 |
| `TR:2e5dfa2f2e3a` | toxic | `safe_to_flag` | 41 | 52 |
| `TR:303d8e539680` | toxic | `safe_to_flag` | 43 | 54 |
| `TR:6406b4103271` | toxic | `safe_to_flag` | 43 | 55 |
| `TR:4ca60eb17b47` | toxic | `safe_to_flag` | 42 | 59 |
| `TR:f2242a603dd4` | toxic | `safe_to_flag` | 45 | 50 |
| `TR:03c3d0c87c2a` | toxic | `safe_to_flag` | 40 | 55 |
| `TR:df195cc7a57b` | toxic | `safe_to_flag` | 29 | 51 |
| `TR:f0d4efecde90` | toxic | `safe_to_flag` | 18 | 54 |
| `TR:3bc132695e52` | toxic | `safe_to_flag` | 11 | 54 |
| `TR:28cccfae8b7f` | toxic | `safe_to_flag` | 26 | 54 |
| `TR:b1c6f6110607` | toxic | `safe_to_flag` | 47 | 78 |
| `TR:75e5280a33c1` | toxic | `safe_to_flag` | 23 | 51 |
| `TR:51b0d2c932ed` | toxic | `safe_to_flag` | 22 | 56 |
| `TR:92f15bccc789` | toxic | `flag_to_safe` | 50 | 36 |
| `TR:7679b7b1e596` | toxic | `flag_to_safe` | 55 | 45 |
