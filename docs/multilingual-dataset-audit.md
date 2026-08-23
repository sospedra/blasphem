# Multilingual toxicity dataset audit

Date: 2026-09-02

## Verdict

No single public dataset can support this detector.

Use native user-generated content for training. Use clean-heavy sources for threshold calibration. Keep independent functional tests sealed.

[TextDetox](https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset) is useful training data. It is not a valid final benchmark because it combines different sources and labels.

The public data can start the work. A production claim still needs a product-owned clean-message panel for every language.

## Recommended sources

`Train` means model fitting. `Supplement` means category or domain coverage. `Seal` means no feature or threshold work may inspect the rows.

| Language | Train | Supplement or hard negatives | Sealed public evaluation | Main limit |
|---|---|---|---|---|
| EN | [OLID](https://sites.google.com/site/offensevalsharedtask/olid), 14,100 Twitter posts | [Civil Comments](https://huggingface.co/datasets/google/civil_comments), 1,999,514 news comments | [HateCheck](https://github.com/paul-rottger/hatecheck-data), 3,728 functional cases | OLID is topic sampled. Civil Comments is longer than chat. |
| ZH | [COLD](https://github.com/thu-coai/COLDataset), 37,480 Weibo and Zhihu comments | [ToxiCN](https://github.com/DUT-lujunyu/ToxiCN), 12,011 Zhihu and Tieba posts | COLD test, 5,323 rows. [Multilingual HateCheck](https://github.com/rewire-online/multilingual-hatecheck) adds Mandarin functions. | The data is mainly Simplified Chinese. It has no Pinyin slice. |
| ES | [OffendES](https://huggingface.co/datasets/fmplaza/offendes), 30,416 public rows | [HatEval](https://huggingface.co/datasets/valeriobasile/HatEval), 6,600 Spanish tweets | [SocialTOX](https://huggingface.co/datasets/gplsi/SocialTOX) test, 968 rows. MHC adds 3,745 functions. | OffendES has low agreement and strong YouTube concentration. |
| AR | [OSACT4](https://github.com/motazsaad/arabic-hatespeech-data/tree/master/OSACT4), 10,000 tweets | [MPOLD](https://github.com/shammur/Arabic-Offensive-Multi-Platform-SocialMedia-Comment-Dataset), 4,000 multi-platform comments | MHC Arabic, 3,570 cases. It includes 133 Arabizi cases. | The releases do not identify MSA and dialect rows. |
| ID | [IndoToxic2024](https://github.com/izzako/IndoToxic2024), 43,692 social posts | [Ibrohim-Budi](https://github.com/okkyibrohim/id-multi-label-hate-speech-and-abusive-language-detection), 13,169 tweets | [Alfina](https://github.com/ialfina/id-hatespeech-detection), 713 unanimous-label tweets | Election and vulnerable-group queries dominate the sources. |
| PT | [ToLD-Br](https://github.com/joaoaleite/ToLD-Br), 21,000 tweets | [HateBR](https://github.com/franciellevargas/HateBR), 7,000 Instagram comments | MHC Portuguese, 3,691 cases. Use Jigsaw after an overlap check. | Both training sources use Brazilian Portuguese. |
| FR | [MLMA](https://github.com/HKUST-KnowComp/MLMA_hate_speech), 4,014 human-labeled tweets | [TOXIFRENCH](https://huggingface.co/datasets/AxelDlv00/ToxiFrench), 52,274 clean-heavy train rows | TOXIFRENCH test, 1,388 rows. MHC adds 3,718 functions. | TOXIFRENCH used automatic labels for about 90 percent of rows. |
| HI | [CONSTRAINT 2021](https://constraint-shared-task-2021.github.io/), 8,192 social posts | [Hindi-English hate corpus](https://github.com/deepanshu1995/HateSpeech-Hindi-English-Code-Mixed-Social-Media-Text), 4,575 tweets | [HateCheckHIn](https://github.com/hate-alert/HateCheckHIn), 5,884 cases | The sources split Devanagari and Roman Hindi coverage. |
| RU | [Russian Toxic Comments](https://github.com/sismetanin/toxic-comments-detection-in-russian), 14,412 forum comments | [RuEthnoHate](https://github.com/hse-scila/ethnohate-project), 12,339 unique texts | [MERA ruHateSpeech](https://huggingface.co/datasets/MERA-evaluation/MERA/tree/main/data/ruhatespeech), 265 pairwise cases | The primary source has weak annotation documentation. |
| JA | [LLM-jp Toxicity v2](https://gitlab.llm-jp.nii.ac.jp/datasets/llm-jp-toxicity-dataset-v2), 3,847 web documents | [Japanese Toxic Dataset](https://github.com/inspection-ai/japanese-toxic-dataset), 437 public snippets | [Court-case posts](https://github.com/horshohei/japanese-offensive-language-from-court-case), 625 cases. RTP-LX adds Japanese tests. | No large public source combines native UGC and strong human labels. |
| DE | [GermEval 2018](https://github.com/uds-lsv/GermEval-2018-Data), 8,541 tweets | [DeTox](https://github.com/hdaSprachtechnologie/detox), 10,278 annotated comments | MHC German, 3,645 cases. Use Jigsaw after an overlap check. | DeTox public tables can require a text-access request. |
| TR | [OffensEval-TR](https://huggingface.co/datasets/coltekin/offenseval2020_tr), 31,756 train tweets | [Toraman v2](https://github.com/avaapm/hatespeech), 60,310 tweet identifiers | [Jigsaw Multilingual](https://www.kaggle.com/c/jigsaw-multilingual-toxic-comment-classification/data) Turkish test. RTP-LX adds functions. | Toraman requires tweet hydration. The current project opened the OffensEval test. |
| VI | [ViHSD](https://github.com/sonlam1102/vihsd), 33,400 Facebook and YouTube comments | [UIT-ViCTSD](https://github.com/tarudesu/ViCTSD), 10,000 news comments | UIT-ViCTSD test, 1,000 rows | The sources do not define an unaccented Vietnamese slice. |
| KO | [K-MHaS](https://github.com/adlnlp/K-MHaS), 78,977 train comments | [KOLD](https://github.com/boychaboy/KOLD), 40,429 Naver and YouTube comments | [UnSmile](https://github.com/smilegate-ai/korean_unsmile_dataset) validation, 3,737 rows | The current project opened the K-MHaS test. Romanized Korean has no public test slice. |
| IT | [Italian Hate Speech Corpus](https://github.com/msang/hate-speech-corpus), 6,928 tweets | [AMI 2018](https://github.com/evalita2018/data/tree/master/AMI), 5,000 tweets | MHC Italian, 3,690 cases | The primary release requires tweet rehydration. The target groups are narrow. |

## Cross-language tests

[Multilingual HateCheck](https://aclanthology.org/2022.woah-1.15/) covers AR, ZH, DE, FR, HI, IT, PT, and ES. It tests 34 language functions across ten languages.

[HateCheckHIn](https://aclanthology.org/2022.lrec-1.575/) covers Devanagari, Roman Hindi, mixed scripts, and Hindi-English code switching.

[RTP-LX](https://github.com/microsoft/RTP-LX) has over 1,000 annotated items per locale across 38 languages. It covers all 15 project languages.

RTP-LX contains translated prompts and synthetic completions. Use it for cultural and functional checks, not production-rate estimates.

[Jigsaw Multilingual](https://www.kaggle.com/c/jigsaw-multilingual-toxic-comment-classification/data) has native evaluation comments for ES, IT, TR, RU, FR, and PT.

Run exact-text and normalized-text overlap checks before using Jigsaw. TextDetox can contain rows from the same upstream sources.

## Label mapping

Map only these source labels to the positive nudge class.

- Direct threat or incitement to violence.
- Directed insult, harassment, or abuse.
- Hate or identity attack.
- Directed profanity or a self-harm command.

Keep generic negative sentiment out of the positive class. Keep reported abuse, quotations, counterspeech, and non-directed profanity as negative controls.

Do not map fake news, political disagreement, sexual content, or illegal-topic labels without direct abuse.

## Required data controls

1. Group exact and normalized duplicates before any split.
2. Group rows from the same post, thread, or source identifier.
3. Keep every functional suite outside training and threshold selection.
4. Record each source label before mapping it to the product label.
5. Use translated or generated rows only as training augmentation.

## Precision evidence

The public corpora do not match pre-send message prevalence. Balanced tests can therefore report misleading precision.

Create at least 10,000 clean, native, product-like messages per language. Keep these messages sealed until the threshold is frozen.

At 1 percent toxic prevalence, 30 percent recall and a 3 percent false-warning rate produce about 9.2 percent precision.

At 1 percent toxic prevalence and 30 percent recall, 90 percent precision requires at most a 0.034 percent false-warning rate.

Ten thousand clean cases with zero warnings gives a one-sided 95 percent upper bound near 0.03 percent.

## Current project overlap

The current benchmark already opened OffensEval-TR test, ViHOS test, and K-MHaS test. These files cannot support a new untouched-test claim.

TextDetox contains upstream material from several recommended sources. Deduplicate all imported rows against the current TextDetox corpus.

ViHOS derives from ViHSD. Do not place ViHOS and ViHSD duplicates across training and evaluation.

## Acquisition order

1. Add COLD, OffendES, IndoToxic2024, ViHSD, KOLD, and UnSmile.
2. Add MLMA, OSACT4, MPOLD, CONSTRAINT, and LLM-jp v2.
3. Add the smaller category supplements after the label mapping exists.
4. Freeze the public evaluation suites before model work.
5. Build the product-owned clean panels before any accuracy claim.
