use std::{env, fs, path::PathBuf};

fn main() {
    risc0_build::embed_methods();
    write_zk_auth_image_id_sol();
}

fn write_zk_auth_image_id_sol() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let methods_rs = fs::read_to_string(out_dir.join("methods.rs")).expect("read generated methods.rs");
    let id = parse_method_id(&methods_rs, "ZK_AUTH_METHOD_ID").expect("find ZK_AUTH_METHOD_ID");
    let image_id = method_id_to_bytes32_hex(&id);

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    let generated_dir = manifest_dir
        .parent()
        .expect("methods crate has workspace parent")
        .join("contracts")
        .join("generated");
    fs::create_dir_all(&generated_dir).expect("create contracts/generated");
    fs::write(
        generated_dir.join("ZkAuthImageID.sol"),
        format!(
            "// SPDX-License-Identifier: MIT\npragma solidity ^0.8.20;\n\nlibrary ZkAuthImageID {{\n    bytes32 internal constant IMAGE_ID = {image_id};\n}}\n"
        ),
    )
    .expect("write ZkAuthImageID.sol");
}

fn parse_method_id(source: &str, name: &str) -> Option<[u32; 8]> {
    let marker = format!("pub const {name}: [u32; 8] = [");
    let start = source.find(&marker)? + marker.len();
    let end = source[start..].find("];")? + start;
    let values = source[start..end]
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    values.try_into().ok()
}

fn method_id_to_bytes32_hex(id: &[u32; 8]) -> String {
    let mut bytes = Vec::with_capacity(32);
    for word in id {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    format!("0x{}", hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
