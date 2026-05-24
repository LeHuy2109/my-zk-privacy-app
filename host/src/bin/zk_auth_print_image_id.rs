use anyhow::Result;
use host::zk_auth::zk_auth_image_id_hex;

fn main() -> Result<()> {
    println!("{}", zk_auth_image_id_hex());
    Ok(())
}
