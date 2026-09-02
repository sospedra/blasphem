# ELDC 15-profile artifact format

`eldc-15-v1.bin` uses little-endian integers and floats. The file has no padding.

The 76-byte header contains these fields in order.

| Bytes | Type | Value |
| --- | --- | --- |
| 0..8 | `[u8; 8]` | `ELDC15\0\0` |
| 8..12 | `u32` | Format version `1` |
| 12..16 | `u32` | Language count `15` |
| 16..20 | `u32` | Hash table slot count |
| 20..24 | `u32` | Score blob item count |
| 24..28 | `u32` | Letter bit table byte count `8192` |
| 28..32 | `u32` | CJK bit table byte count `8192` |
| 32..36 | `u32` | Lowercase table item count `1920` |
| 36..76 | ASCII | The 40-byte upstream commit |

The header is followed by these sections in order.

1. The file stores 15 `f32` average scores in compact language order.
2. The file stores 8192 letter bit table bytes.
3. The file stores 8192 CJK bit table bytes.
4. The file stores 1920 lowercase code points as `u16` values.
5. The file stores each hash slot as a fingerprint `u32` and metadata `u32`.
6. The file stores each packed score as a `u32` value.

Hash metadata uses its upper byte for the score count. Its lower 24 bits store the blob offset.

A packed score uses its low byte for the compact language index. Its upper 24 bits store the truncated `f32` weight.

The parser rejects an invalid header, section boundary, slot range, language index, truncated file, or trailing byte.
