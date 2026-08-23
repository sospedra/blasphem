use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_COMMIT: &str = "a0301db809ff2e48a418018aa5359fb0c4354eb8";
const TABLE_LENGTH: usize = 2_097_152;
const LANGUAGE_COUNT: usize = 15;
const LETTER_TABLE_LENGTH: usize = 8_192;
const CJK_TABLE_LENGTH: usize = 8_192;
const LOWERCASE_TABLE_LENGTH: usize = 1_920;

const PINNED_FILES: [(&str, &str); 4] = [
    (
        "large_db.h",
        "4f9f3d9741e5f594b0a50da9bf1d26cfba2b8f049a1b75627114a6cc9c0dfe64",
    ),
    (
        "eld_unicode_bits.h",
        "e620b9feb08eb32ce751a7148a51b19c5eb2774d2dff74f5dd2d1363184df23b",
    ),
    (
        "eld_tolower.h",
        "97722a4d9765e609631ce527ff42b27a4e589d7e673d17e8bf1da68068da1d2b",
    ),
    (
        "eld_unicode.h",
        "26b6b645823f81796dcdafdf8eedb41299d769d8c06579eab9ec4ffa3e519cf0",
    ),
];

const SELECTED_UPSTREAM_INDEXES: [usize; LANGUAGE_COUNT] =
    [1, 9, 11, 12, 17, 20, 25, 26, 29, 36, 42, 44, 54, 57, 59];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Slot {
    fingerprint: u32,
    metadata: u32,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("blasphem-language-model: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os();
    let executable = arguments.next().unwrap_or_default();
    let source = arguments.next().map(PathBuf::from);
    let output = arguments.next().map(PathBuf::from);
    if source.is_none() || output.is_none() || arguments.next().is_some() {
        return Err(format!(
            "usage: {} SOURCE_DIRECTORY OUTPUT_FILE",
            Path::new(&executable).display()
        ));
    }
    let source = source.expect("checked above");
    let output = output.expect("checked above");

    let mut sources = Vec::with_capacity(PINNED_FILES.len());
    for (name, digest) in PINNED_FILES {
        sources.push(read_verified_source(&source, name, digest)?);
    }
    let large_database = &sources[0];
    let unicode_bits = &sources[1];
    let lowercase_source = &sources[2];

    let averages = parse_f32_array(large_database, "ELD_avg_score")?;
    if averages.len() != 60 {
        return Err(format!("expected 60 averages, found {}", averages.len()));
    }
    let original_slots = parse_slots(large_database, "ELD_hashtable")?;
    if original_slots.len() != TABLE_LENGTH {
        return Err(format!(
            "expected {TABLE_LENGTH} hash slots, found {}",
            original_slots.len()
        ));
    }
    let original_blob = parse_u32_array(large_database, "ELD_blob")?;
    let letter_bits = parse_u32_array(unicode_bits, "LETTER_BITS")?;
    if letter_bits.len() != LETTER_TABLE_LENGTH {
        return Err(format!(
            "expected {LETTER_TABLE_LENGTH} letter bytes, found {}",
            letter_bits.len()
        ));
    }
    let cjk_bits = parse_u32_array(unicode_bits, "CJK_BITS")?;
    if cjk_bits.len() != CJK_TABLE_LENGTH {
        return Err(format!(
            "expected {CJK_TABLE_LENGTH} CJK bytes, found {}",
            cjk_bits.len()
        ));
    }
    let lowercase = parse_u32_array(lowercase_source, "TOLOWER_BMP2")?;
    if lowercase.len() != LOWERCASE_TABLE_LENGTH {
        return Err(format!(
            "expected {LOWERCASE_TABLE_LENGTH} lowercase values, found {}",
            lowercase.len()
        ));
    }

    let (slots, blob) = filter_slots(&original_slots, &original_blob)?;
    let mut artifact = Vec::with_capacity(
        76 + LANGUAGE_COUNT * 4
            + LETTER_TABLE_LENGTH
            + CJK_TABLE_LENGTH
            + LOWERCASE_TABLE_LENGTH * 2
            + slots.len() * 8
            + blob.len() * 4,
    );
    artifact.extend_from_slice(b"BLASPHEM");
    write_u32(&mut artifact, 1);
    write_u32(&mut artifact, LANGUAGE_COUNT as u32);
    write_u32(&mut artifact, slots.len() as u32);
    write_u32(&mut artifact, blob.len() as u32);
    write_u32(&mut artifact, letter_bits.len() as u32);
    write_u32(&mut artifact, cjk_bits.len() as u32);
    write_u32(&mut artifact, lowercase.len() as u32);
    artifact.extend_from_slice(SOURCE_COMMIT.as_bytes());
    for upstream_index in SELECTED_UPSTREAM_INDEXES {
        write_u32(&mut artifact, averages[upstream_index].to_bits());
    }
    for value in letter_bits {
        artifact.push(
            u8::try_from(value).map_err(|_| format!("letter table value {value} exceeds u8"))?,
        );
    }
    for value in cjk_bits {
        artifact
            .push(u8::try_from(value).map_err(|_| format!("CJK table value {value} exceeds u8"))?);
    }
    for value in lowercase {
        let value = u16::try_from(value)
            .map_err(|_| format!("lowercase table value {value} exceeds u16"))?;
        artifact.extend_from_slice(&value.to_le_bytes());
    }
    for slot in slots {
        write_u32(&mut artifact, slot.fingerprint);
        write_u32(&mut artifact, slot.metadata);
    }
    for score in blob {
        write_u32(&mut artifact, score);
    }

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(&output, &artifact)
        .map_err(|error| format!("cannot write {}: {error}", output.display()))?;
    let digest = sha256_hex(&artifact);
    println!(
        "wrote {} bytes to {} sha256={digest}",
        artifact.len(),
        output.display()
    );
    Ok(())
}

fn read_verified_source(root: &Path, name: &str, expected_digest: &str) -> Result<String, String> {
    let path = root.join(name);
    let bytes =
        fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let actual_digest = sha256_hex(&bytes);
    if actual_digest != expected_digest {
        return Err(format!(
            "{} has SHA-256 {actual_digest}, expected {expected_digest}",
            path.display()
        ));
    }
    String::from_utf8(bytes).map_err(|error| format!("{} is not UTF-8: {error}", path.display()))
}

fn array_body<'a>(source: &'a str, name: &str) -> Result<&'a str, String> {
    let name_offset = source
        .find(name)
        .ok_or_else(|| format!("array {name} is missing"))?;
    let declaration = &source[name_offset + name.len()..];
    let equals_offset = declaration
        .find('=')
        .ok_or_else(|| format!("array {name} has no initializer"))?;
    let initializer = &declaration[equals_offset + 1..];
    let open_offset = initializer
        .find('{')
        .ok_or_else(|| format!("array {name} has no opening brace"))?;
    let body_start = open_offset + 1;
    let bytes = initializer.as_bytes();
    let mut depth = 1_usize;
    for (offset, byte) in bytes[body_start..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&initializer[body_start..body_start + offset]);
                }
            }
            _ => {}
        }
    }
    Err(format!("array {name} has no closing brace"))
}

fn parse_u32_array(source: &str, name: &str) -> Result<Vec<u32>, String> {
    array_body(source, name)?
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(parse_u32)
        .collect()
}

fn parse_f32_array(source: &str, name: &str) -> Result<Vec<f32>, String> {
    array_body(source, name)?
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| {
            token
                .trim_end_matches(['f', 'F'])
                .parse::<f32>()
                .map_err(|error| format!("invalid float {token}: {error}"))
        })
        .collect()
}

fn parse_u32(token: &str) -> Result<u32, String> {
    let token = token.trim().trim_end_matches(['u', 'U']);
    if let Some(hex) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        u32::from_str_radix(hex, 16).map_err(|error| format!("invalid integer {token}: {error}"))
    } else {
        token
            .parse::<u32>()
            .map_err(|error| format!("invalid integer {token}: {error}"))
    }
}

fn parse_slots(source: &str, name: &str) -> Result<Vec<Slot>, String> {
    let body = array_body(source, name)?;
    let mut slots = Vec::new();
    let mut remaining = body;
    while let Some(open_offset) = remaining.find('{') {
        remaining = &remaining[open_offset + 1..];
        let close_offset = remaining
            .find('}')
            .ok_or_else(|| format!("slot array {name} has an unclosed slot"))?;
        let slot_body = &remaining[..close_offset];
        let values: Vec<_> = slot_body
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(parse_u32)
            .collect::<Result<_, _>>()?;
        let slot = match values.as_slice() {
            [0] => Slot {
                fingerprint: 0,
                metadata: 0,
            },
            [fingerprint, metadata] => Slot {
                fingerprint: *fingerprint,
                metadata: *metadata,
            },
            _ => return Err(format!("slot array {name} contains an invalid slot")),
        };
        slots.push(slot);
        remaining = &remaining[close_offset + 1..];
    }
    Ok(slots)
}

fn compact_index(upstream_index: u8) -> Option<u8> {
    SELECTED_UPSTREAM_INDEXES
        .iter()
        .position(|index| *index == upstream_index as usize)
        .map(|index| index as u8)
}

fn filter_slots(slots: &[Slot], blob: &[u32]) -> Result<(Vec<Slot>, Vec<u32>), String> {
    let mut filtered_slots = Vec::with_capacity(slots.len());
    let mut filtered_blob = Vec::new();
    for (slot_index, slot) in slots.iter().enumerate() {
        if slot.fingerprint == 0 {
            if slot.metadata != 0 {
                return Err(format!("empty slot {slot_index} has metadata"));
            }
            filtered_slots.push(*slot);
            continue;
        }
        let original_offset = (slot.metadata & 0x00ff_ffff) as usize;
        let original_count = (slot.metadata >> 24) as usize;
        let original_end = original_offset
            .checked_add(original_count)
            .ok_or_else(|| format!("slot {slot_index} blob range overflows"))?;
        let original_scores = blob
            .get(original_offset..original_end)
            .ok_or_else(|| format!("slot {slot_index} exceeds the source blob"))?;
        let filtered_offset = filtered_blob.len();
        if filtered_offset > 0x00ff_ffff {
            return Err("the filtered blob exceeds the 24-bit offset limit".to_owned());
        }
        for score in original_scores {
            if let Some(index) = compact_index(*score as u8) {
                filtered_blob.push((*score & 0xffff_ff00) | u32::from(index));
            }
        }
        let filtered_count = filtered_blob.len() - filtered_offset;
        let metadata = (u32::try_from(filtered_count)
            .map_err(|_| format!("slot {slot_index} score count exceeds u32"))?
            << 24)
            | filtered_offset as u32;
        filtered_slots.push(Slot {
            fingerprint: slot.fingerprint,
            metadata,
        });
    }
    Ok((filtered_slots, filtered_blob))
}

fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn sha256_hex(input: &[u8]) -> String {
    const INITIAL: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const ROUND: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let bit_length = (input.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(input.len() + 72);
    padded.extend_from_slice(input);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());

    let mut state = INITIAL;
    for chunk in padded.chunks_exact(64) {
        let mut schedule = [0_u32; 64];
        let mut index = 0;
        while index < 16 {
            let offset = index * 4;
            schedule[index] = u32::from_be_bytes(
                chunk[offset..offset + 4]
                    .try_into()
                    .expect("four-byte SHA-256 word"),
            );
            index += 1;
        }
        while index < 64 {
            let first = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let second = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(first)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(second);
            index += 1;
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for index in 0..64 {
            let upper = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ (!e & g);
            let first = h
                .wrapping_add(upper)
                .wrapping_add(choose)
                .wrapping_add(ROUND[index])
                .wrapping_add(schedule[index]);
            let lower = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let second = lower.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(first);
            d = c;
            c = b;
            b = a;
            a = first.wrapping_add(second);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(64);
    for word in state {
        for byte in word.to_be_bytes() {
            result.push(HEX[usize::from(byte >> 4)] as char);
            result.push(HEX[usize::from(byte & 0x0f)] as char);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{Slot, filter_slots, parse_slots, parse_u32_array, sha256_hex};

    #[test]
    fn sha256_matches_the_standard_abc_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn numeric_parser_accepts_generated_header_literals() {
        assert_eq!(
            parse_u32_array("static const uint32_t X[] = { 0x10u, 32, 0X2AU };", "X").unwrap(),
            vec![16, 32, 42]
        );
    }

    #[test]
    fn slot_parser_keeps_empty_and_occupied_slots() {
        assert_eq!(
            parse_slots("static const Slot X[2] = {{0},{0xABu,0x1000002u}};", "X").unwrap(),
            vec![
                Slot {
                    fingerprint: 0,
                    metadata: 0,
                },
                Slot {
                    fingerprint: 0xab,
                    metadata: 0x0100_0002,
                },
            ]
        );
    }

    #[test]
    fn profile_filter_rewrites_indexes_and_keeps_fingerprints() {
        let scores = [
            1.5_f32.to_bits() | 1,
            2.0_f32.to_bits(),
            3.0_f32.to_bits() | 59,
        ];
        let slots = [
            Slot {
                fingerprint: 7,
                metadata: 3 << 24,
            },
            Slot {
                fingerprint: 8,
                metadata: 0,
            },
            Slot {
                fingerprint: 0,
                metadata: 0,
            },
        ];
        let (filtered_slots, filtered_scores) = filter_slots(&slots, &scores).unwrap();
        assert_eq!(filtered_slots[0].fingerprint, 7);
        assert_eq!(filtered_slots[0].metadata, 2 << 24);
        assert_eq!(filtered_slots[1].fingerprint, 8);
        assert_eq!(filtered_slots[1].metadata, 2);
        assert_eq!(filtered_slots[2].fingerprint, 0);
        assert_eq!(filtered_slots[2].metadata, 0);
        assert_eq!(filtered_scores, [1.5_f32.to_bits(), 3.0_f32.to_bits() | 14]);
    }
}
